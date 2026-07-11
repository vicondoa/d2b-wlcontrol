//! Action availability gating and argv/intent planning.

use crate::config::Config;
use crate::model::{
    ActionAvailability, ActionKind, AudioChannel, AudioEnforcementPosture, AuthRole, Connectivity,
    LauncherItemKind, PlannedAction, RealmLauncherEntry, RuntimeState, SocketIntent, Unavailable,
    Vm, WlState, WorkloadAvailability,
};

/// Returns `Some(reason)` when `action` cannot currently be invoked.
pub fn block_reason(action: &ActionKind, state: &WlState) -> Option<Unavailable> {
    // Display-only actions are always available.
    match action {
        ActionKind::OpenControlCenter
        | ActionKind::OpenObservability
        | ActionKind::CycleDisplay
        | ActionKind::Refresh => return None,
        _ => {}
    }

    if state.connectivity == Connectivity::DaemonDown {
        return Some(Unavailable::DaemonDown);
    }

    let required = required_role(action);
    if !role_satisfies(state.role, required) {
        return Some(Unavailable::InsufficientRole { required });
    }
    if let Some(vm) = action_target_vm(action) {
        if unsafe_local_workload_for_vm(state, vm).is_some() {
            return Some(Unavailable::Blocked {
                detail: "unsafe-local workloads expose launcher items only; there is no VM isolation or VM control surface".into(),
            });
        }
    }
    if let Some(reason) = capability_block(action, state) {
        return Some(reason);
    }

    match action {
        ActionKind::Start { vm } => running_vm(state, vm)
            .filter(|v| v.state == RuntimeState::Running)
            .map(|_| Unavailable::VmState {
                detail: "VM is already running".into(),
            }),
        ActionKind::Stop { vm }
        | ActionKind::ForceStop { vm }
        | ActionKind::Restart { vm }
        | ActionKind::Switch { vm } => running_vm(state, vm)
            .filter(|v| v.state == RuntimeState::Stopped)
            .map(|_| Unavailable::VmState {
                detail: "VM is not running".into(),
            }),
        ActionKind::LaunchTerminal { vm } => running_vm(state, vm)
            .filter(|v| v.state != RuntimeState::Running)
            .map(|_| Unavailable::VmState {
                detail: "start the VM before opening a terminal".into(),
            }),
        ActionKind::QuickLaunch { vm, .. } => running_vm(state, vm)
            .filter(|v| v.state != RuntimeState::Running)
            .map(|_| Unavailable::VmState {
                detail: "start the VM before using quick launch".into(),
            }),
        ActionKind::UsbAttach { vm, bus_id } => usb_attach_block(state, vm, bus_id),
        ActionKind::UsbDetach { vm, bus_id } => usb_detach_block(state, vm, bus_id),
        ActionKind::AudioSpeakerVolume { level_percent, .. }
        | ActionKind::AudioMicGain { level_percent, .. }
            if *level_percent > 100 =>
        {
            Some(Unavailable::Blocked {
                detail: "audio level must be between 0 and 100".into(),
            })
        }
        ActionKind::AudioMic { vm, .. }
        | ActionKind::AudioSpeaker { vm, .. }
        | ActionKind::AudioSpeakerVolume { vm, .. }
        | ActionKind::AudioMicGain { vm, .. }
        | ActionKind::AudioOff { vm } => audio_block(state, vm),
        ActionKind::StoreVerify { .. } | ActionKind::Build { .. } | ActionKind::Boot { .. } => None,
        ActionKind::RealmWorkloadLaunch { .. } => Some(Unavailable::Blocked {
            detail:
                "legacy realm launcher actions are unavailable; refresh public workload inventory"
                    .into(),
        }),
        ActionKind::WorkloadLaunch {
            target,
            item_id,
            item_kind,
        } => workload_launch_block(state, target, item_id, *item_kind),
        _ => None,
    }
}

fn workload_launch_block(
    state: &WlState,
    target: &str,
    item_id: &str,
    item_kind: LauncherItemKind,
) -> Option<Unavailable> {
    let Some(workload) = workload_by_target(state, target) else {
        return Some(Unavailable::Blocked {
            detail: "workload is not present in the public inventory".into(),
        });
    };
    let Some(item) = workload
        .launcher_items
        .iter()
        .find(|item| item.id == item_id)
    else {
        return Some(Unavailable::Blocked {
            detail: "launcher item is not present in the public inventory".into(),
        });
    };
    if item.kind != item_kind || item_kind == LauncherItemKind::Unknown {
        return Some(Unavailable::Blocked {
            detail: "launcher item kind does not match the public inventory".into(),
        });
    }
    if workload.availability == WorkloadAvailability::Ready {
        None
    } else {
        Some(Unavailable::Blocked {
            detail: workload
                .availability
                .remediation()
                .unwrap_or("Workload is unavailable.")
                .to_owned(),
        })
    }
}

fn capability_block(action: &ActionKind, state: &WlState) -> Option<Unavailable> {
    let vm = action_target_vm(action)?;
    let target = running_vm(state, vm)?;
    let supported = match action {
        ActionKind::Start { .. } => target.capabilities.start,
        ActionKind::Stop { .. } | ActionKind::ForceStop { .. } => target.capabilities.stop,
        ActionKind::Restart { .. } => target.capabilities.restart,
        ActionKind::Switch { .. } => target.capabilities.switch,
        ActionKind::Build { .. } => target.capabilities.build,
        ActionKind::Boot { .. } => target.capabilities.boot,
        ActionKind::UsbAttach { .. } | ActionKind::UsbDetach { .. } => {
            target.capabilities.usb_hotplug
        }
        ActionKind::StoreVerify { .. } => target.capabilities.store_verify,
        ActionKind::LaunchTerminal { .. } | ActionKind::QuickLaunch { .. } => {
            target.capabilities.terminal
        }
        ActionKind::AudioMic { .. }
        | ActionKind::AudioSpeaker { .. }
        | ActionKind::AudioSpeakerVolume { .. }
        | ActionKind::AudioMicGain { .. }
        | ActionKind::AudioOff { .. } => target.features.audio || target.audio.is_some(),
        _ => true,
    };
    (!supported).then(|| Unavailable::Blocked {
        detail: "unsupported by this VM runtime".into(),
    })
}

fn action_target_vm(action: &ActionKind) -> Option<&str> {
    match action {
        ActionKind::Start { vm }
        | ActionKind::Stop { vm }
        | ActionKind::ForceStop { vm }
        | ActionKind::Restart { vm }
        | ActionKind::Switch { vm }
        | ActionKind::Build { vm }
        | ActionKind::Boot { vm }
        | ActionKind::QuickLaunch { vm, .. }
        | ActionKind::UsbAttach { vm, .. }
        | ActionKind::UsbDetach { vm, .. }
        | ActionKind::StoreVerify { vm }
        | ActionKind::LaunchTerminal { vm }
        | ActionKind::AudioMic { vm, .. }
        | ActionKind::AudioSpeaker { vm, .. }
        | ActionKind::AudioSpeakerVolume { vm, .. }
        | ActionKind::AudioMicGain { vm, .. }
        | ActionKind::AudioOff { vm } => Some(vm.as_str()),
        ActionKind::Refresh
        | ActionKind::OpenControlCenter
        | ActionKind::OpenObservability
        | ActionKind::CycleDisplay
        | ActionKind::RealmWorkloadLaunch { .. }
        | ActionKind::WorkloadLaunch { .. } => None,
    }
}

/// Return the full per-VM action list the control center can render.
pub fn vm_actions(state: &WlState, config: &Config, vm: &str) -> Vec<ActionAvailability> {
    let mut actions = vec![
        ActionKind::Start { vm: vm.into() },
        ActionKind::Stop { vm: vm.into() },
        ActionKind::ForceStop { vm: vm.into() },
        ActionKind::Restart { vm: vm.into() },
        ActionKind::LaunchTerminal { vm: vm.into() },
        ActionKind::StoreVerify { vm: vm.into() },
        ActionKind::Build { vm: vm.into() },
        ActionKind::Boot { vm: vm.into() },
        ActionKind::Switch { vm: vm.into() },
    ];

    if let Some(target) = running_vm(state, vm) {
        for claim in &target.usb {
            actions.push(ActionKind::UsbAttach {
                vm: vm.into(),
                bus_id: claim.bus_id.clone(),
            });
            actions.push(ActionKind::UsbDetach {
                vm: vm.into(),
                bus_id: claim.bus_id.clone(),
            });
        }
    }

    if running_vm(state, vm).is_some_and(|target| target.features.audio || target.audio.is_some()) {
        actions.extend([
            ActionKind::AudioMic {
                vm: vm.into(),
                on: true,
            },
            ActionKind::AudioSpeaker {
                vm: vm.into(),
                on: true,
            },
            ActionKind::AudioSpeakerVolume {
                vm: vm.into(),
                level_percent: 80,
            },
            ActionKind::AudioMicGain {
                vm: vm.into(),
                level_percent: 50,
            },
            ActionKind::AudioOff { vm: vm.into() },
        ]);
    }

    actions
        .into_iter()
        .map(|action| availability(action, state, config))
        .collect()
}

/// Plan a concrete dispatch for an action, or return why it is blocked.
pub fn plan(
    action: &ActionKind,
    state: &WlState,
    config: &Config,
) -> Result<PlannedAction, Unavailable> {
    if let Some(reason) = block_reason(action, state) {
        return Err(reason);
    }
    if let Some(reason) = config_block_reason(action, config) {
        return Err(reason);
    }

    let dispatch = match action {
        ActionKind::Start { vm } => socket(SocketIntent::VmStart { vm: vm.clone() }),
        ActionKind::Stop { vm } => socket(SocketIntent::VmStop {
            vm: vm.clone(),
            force: false,
        }),
        ActionKind::ForceStop { vm } => socket(SocketIntent::VmStop {
            vm: vm.clone(),
            force: true,
        }),
        ActionKind::Restart { vm } => socket(SocketIntent::VmRestart { vm: vm.clone() }),
        ActionKind::Switch { vm } => socket(SocketIntent::Switch { vm: vm.clone() }),
        ActionKind::Build { vm } => build_argv(vm),
        ActionKind::Boot { vm } => socket(SocketIntent::Boot { vm: vm.clone() }),
        ActionKind::UsbAttach { vm, bus_id } => socket(SocketIntent::UsbAttach {
            vm: vm.clone(),
            bus_id: bus_id.clone(),
        }),
        ActionKind::UsbDetach { vm, bus_id } => socket(SocketIntent::UsbDetach {
            vm: vm.clone(),
            bus_id: bus_id.clone(),
        }),
        ActionKind::StoreVerify { vm } => socket(SocketIntent::StoreVerify { vm: vm.clone() }),
        ActionKind::Refresh => socket(SocketIntent::List),
        ActionKind::LaunchTerminal { vm } => terminal_argv(vm, config),
        ActionKind::QuickLaunch { vm, id } => quick_launch_argv(vm, id, config)?,
        ActionKind::OpenObservability => observability_argv(config),
        ActionKind::AudioMic { vm, on } => socket(SocketIntent::AudioMute {
            vm: vm.clone(),
            channel: AudioChannel::Microphone,
            mute: !on,
        }),
        ActionKind::AudioSpeaker { vm, on } => socket(SocketIntent::AudioMute {
            vm: vm.clone(),
            channel: AudioChannel::Speaker,
            mute: !on,
        }),
        ActionKind::AudioSpeakerVolume { vm, level_percent } => {
            socket(SocketIntent::AudioSetVolume {
                vm: vm.clone(),
                channel: AudioChannel::Speaker,
                level_percent: *level_percent,
            })
        }
        ActionKind::AudioMicGain { vm, level_percent } => socket(SocketIntent::AudioSetVolume {
            vm: vm.clone(),
            channel: AudioChannel::Microphone,
            level_percent: *level_percent,
        }),
        ActionKind::AudioOff { vm } => socket(SocketIntent::AudioOff { vm: vm.clone() }),
        ActionKind::OpenControlCenter | ActionKind::CycleDisplay => {
            // These are handled in-process by the UI/Waybar layers, not as a
            // d2b dispatch; planning them is a no-op socket refresh.
            return Err(Unavailable::Blocked {
                detail: "handled in-process; not a d2b dispatch".into(),
            });
        }
        ActionKind::RealmWorkloadLaunch { .. } => {
            return Err(Unavailable::Blocked {
                detail: "legacy realm launcher actions are unavailable".into(),
            })
        }
        ActionKind::WorkloadLaunch {
            target,
            item_id,
            item_kind,
        } => workload_launch_argv(target, item_id, *item_kind, config),
    };
    Ok(dispatch)
}

fn availability(action: ActionKind, state: &WlState, config: &Config) -> ActionAvailability {
    let unavailable = block_reason(&action, state).or_else(|| config_block_reason(&action, config));
    ActionAvailability {
        action,
        unavailable,
    }
}

fn config_block_reason(action: &ActionKind, config: &Config) -> Option<Unavailable> {
    if matches!(
        action,
        ActionKind::LaunchTerminal { .. }
            | ActionKind::QuickLaunch { .. }
            | ActionKind::WorkloadLaunch {
                item_kind: LauncherItemKind::Shell,
                ..
            }
    ) {
        return config.validate().err().map(|err| Unavailable::Blocked {
            detail: err.to_string(),
        });
    }
    if let ActionKind::QuickLaunch { vm, id } = action {
        if quick_launch_config(vm, id, config).is_none() {
            return Some(Unavailable::Blocked {
                detail: format!("quick launch '{id}' is not configured for {vm}"),
            });
        }
    }
    if matches!(action, ActionKind::OpenObservability) {
        if let Err(err) = config.validate() {
            return Some(Unavailable::Blocked {
                detail: err.to_string(),
            });
        }
        if !config.observability.enabled {
            return Some(Unavailable::Blocked {
                detail: "observability is disabled".into(),
            });
        }
        if config.observability.url.is_none() {
            return Some(Unavailable::Blocked {
                detail: "observability.url is not configured".into(),
            });
        }
    }
    None
}

fn required_role(action: &ActionKind) -> AuthRole {
    match action {
        ActionKind::LaunchTerminal { .. }
        | ActionKind::QuickLaunch { .. }
        | ActionKind::RealmWorkloadLaunch { .. }
        | ActionKind::Start { .. }
        | ActionKind::Stop { .. }
        | ActionKind::ForceStop { .. }
        | ActionKind::Restart { .. }
        | ActionKind::Switch { .. }
        | ActionKind::Boot { .. }
        | ActionKind::UsbAttach { .. }
        | ActionKind::UsbDetach { .. }
        | ActionKind::StoreVerify { .. }
        | ActionKind::AudioMic { .. }
        | ActionKind::AudioSpeaker { .. }
        | ActionKind::AudioSpeakerVolume { .. }
        | ActionKind::AudioMicGain { .. }
        | ActionKind::AudioOff { .. } => AuthRole::Admin,
        ActionKind::Build { .. } | ActionKind::WorkloadLaunch { .. } => AuthRole::Launcher,
        _ => AuthRole::None,
    }
}

fn audio_block(state: &WlState, vm: &str) -> Option<Unavailable> {
    let target = running_vm(state, vm)?;
    let Some(audio) = &target.audio else {
        return Some(Unavailable::Blocked {
            detail: "audio status is not available from this d2b generation".into(),
        });
    };
    if let Some(kind) = &audio.error_kind {
        let detail = audio.remediation.as_ref().map_or_else(
            || format!("audio unavailable: {kind}"),
            |remediation| format!("audio unavailable: {kind}; {remediation}"),
        );
        return Some(Unavailable::Blocked { detail });
    }
    if matches!(
        audio.enforcement,
        AudioEnforcementPosture::Unsupported | AudioEnforcementPosture::Unknown
    ) {
        return Some(Unavailable::Blocked {
            detail: "audio controls are unsupported for this VM runtime".into(),
        });
    }
    None
}

fn role_satisfies(have: AuthRole, need: AuthRole) -> bool {
    rank(have) >= rank(need)
}

fn rank(role: AuthRole) -> u8 {
    match role {
        AuthRole::None => 0,
        AuthRole::Launcher => 1,
        AuthRole::Admin => 2,
    }
}

fn running_vm<'a>(state: &'a WlState, name: &str) -> Option<&'a Vm> {
    state.vms.iter().find(|v| v.name == name)
}

fn workload_by_target<'a>(state: &'a WlState, target: &str) -> Option<&'a RealmLauncherEntry> {
    state
        .realm_groups
        .iter()
        .flat_map(|group| &group.workloads)
        .find(|workload| workload.canonical_target == target)
}

fn unsafe_local_workload_for_vm<'a>(
    state: &'a WlState,
    vm_name: &str,
) -> Option<&'a RealmLauncherEntry> {
    let vm_target = running_vm(state, vm_name).and_then(|vm| vm.canonical_target.as_deref());
    state
        .realm_groups
        .iter()
        .flat_map(|group| &group.workloads)
        .find(|workload| {
            workload.is_unsafe_local()
                && (workload.legacy_vm_name.as_deref() == Some(vm_name)
                    || vm_target == Some(workload.canonical_target.as_str()))
        })
}

fn usb_attach_block(state: &WlState, vm: &str, bus_id: &str) -> Option<Unavailable> {
    let claim = state
        .vms
        .iter()
        .flat_map(|v| v.usb.iter())
        .find(|c| c.bus_id == bus_id);
    match claim {
        Some(c) if c.bound => match &c.owner_vm {
            Some(owner) if owner != vm => Some(Unavailable::UsbOwnedElsewhere {
                owner: owner.clone(),
            }),
            _ => None,
        },
        _ => None,
    }
}

fn usb_detach_block(state: &WlState, vm: &str, bus_id: &str) -> Option<Unavailable> {
    let claim = state
        .vms
        .iter()
        .flat_map(|v| v.usb.iter())
        .find(|c| c.bus_id == bus_id);
    match claim {
        Some(c) if c.bound => match &c.owner_vm {
            Some(owner) if owner == vm => None,
            Some(owner) => Some(Unavailable::UsbOwnedElsewhere {
                owner: owner.clone(),
            }),
            None => None,
        },
        _ => Some(Unavailable::VmState {
            detail: "device is not bound".into(),
        }),
    }
}

fn socket(intent: SocketIntent) -> PlannedAction {
    PlannedAction::Socket { intent }
}

/// Build the argv-only detached terminal launch command. There is no shell
/// string and no interpolation: the d2b exec invocation and guest terminal
/// command are concatenated as discrete argv elements.
fn terminal_argv(vm: &str, config: &Config) -> PlannedAction {
    let mut argv = vec![
        "d2b".to_owned(),
        "vm".to_owned(),
        "exec".to_owned(),
        "-d".to_owned(),
        vm.to_owned(),
        "--".to_owned(),
    ];
    if config.terminal.guest_argv.is_empty() {
        argv.push(config.terminal.guest_shell.clone());
    } else {
        argv.extend(config.terminal.guest_argv.clone());
    }
    PlannedAction::Process { argv, wait: true }
}

fn quick_launch_argv(vm: &str, id: &str, config: &Config) -> Result<PlannedAction, Unavailable> {
    let item = quick_launch_config(vm, id, config).ok_or_else(|| Unavailable::Blocked {
        detail: format!("quick launch '{id}' is not configured for {vm}"),
    })?;
    let mut argv = vec![
        "d2b".to_owned(),
        "vm".to_owned(),
        "exec".to_owned(),
        "-d".to_owned(),
        vm.to_owned(),
        "--".to_owned(),
    ];
    argv.extend(item.guest_argv.clone());
    Ok(PlannedAction::Process { argv, wait: true })
}

fn quick_launch_config<'a>(
    vm: &str,
    id: &str,
    config: &'a Config,
) -> Option<&'a crate::config::QuickLaunchConfig> {
    config
        .quick_launch
        .iter()
        .find(|item| item.vm == vm && item.id == id)
}

fn build_argv(vm: &str) -> PlannedAction {
    PlannedAction::Process {
        argv: vec!["d2b".to_owned(), "build".to_owned(), vm.to_owned()],
        wait: true,
    }
}

fn observability_argv(config: &Config) -> PlannedAction {
    let mut argv = config.observability.browser_argv.clone();
    if let Some(url) = &config.observability.url {
        argv.push(url.clone());
    }
    PlannedAction::Process { argv, wait: false }
}

fn workload_launch_argv(
    target: &str,
    item_id: &str,
    item_kind: LauncherItemKind,
    config: &Config,
) -> PlannedAction {
    let argv = match item_kind {
        LauncherItemKind::Exec => vec![
            "d2b".to_owned(),
            "launch".to_owned(),
            target.to_owned(),
            "--item".to_owned(),
            item_id.to_owned(),
        ],
        LauncherItemKind::Shell => {
            let mut argv = config.terminal.wlterm_argv.clone();
            argv.push(target.to_owned());
            argv.push(item_id.to_owned());
            argv
        }
        LauncherItemKind::Unknown => Vec::new(),
    };
    PlannedAction::Process { argv, wait: true }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AudioChannelState, AudioProviderKind, IsolationPosture, LauncherIcon, LauncherItemSummary,
        RealmGroup, UsbClaim, VmAudioState, WorkloadExecutionPosture, WorkloadProviderKind,
    };

    fn connected_state(role: AuthRole, vms: Vec<Vm>) -> WlState {
        WlState {
            connectivity: Connectivity::Connected,
            role,
            vms,
            stale: false,
            note: None,
            ..Default::default()
        }
    }

    fn vm(name: &str, state: RuntimeState) -> Vm {
        Vm {
            name: name.into(),
            state,
            ..Default::default()
        }
    }

    fn usb_claim(vm: &str, bus_id: &str, bound: bool, owner_vm: Option<&str>) -> UsbClaim {
        UsbClaim {
            vm: vm.into(),
            env: "work".into(),
            bus_id: bus_id.into(),
            bound,
            owner_vm: owner_vm.map(str::to_owned),
        }
    }

    fn workload_state(
        provider_kind: WorkloadProviderKind,
        availability: WorkloadAvailability,
    ) -> WlState {
        let target = "tools.host.d2b";
        let workload = RealmLauncherEntry {
            workload_name: "tools".to_owned(),
            label: "Host Tools".to_owned(),
            canonical_target: target.to_owned(),
            realm_name: "host".to_owned(),
            realm_id: "host".to_owned(),
            provider_kind,
            availability,
            execution_posture: WorkloadExecutionPosture {
                isolation: if provider_kind == WorkloadProviderKind::UnsafeLocal {
                    IsolationPosture::UnsafeLocal
                } else {
                    IsolationPosture::VirtualMachine
                },
                ..Default::default()
            },
            launcher_items: vec![
                LauncherItemSummary {
                    id: "firefox".to_owned(),
                    name: "Firefox".to_owned(),
                    icon: LauncherIcon {
                        id: Some("firefox".to_owned()),
                        name: Some("web-browser".to_owned()),
                    },
                    kind: LauncherItemKind::Exec,
                    graphical: true,
                    capabilities: vec!["configured-launch".to_owned()],
                },
                LauncherItemSummary {
                    id: "terminal".to_owned(),
                    name: "Terminal".to_owned(),
                    icon: LauncherIcon {
                        id: None,
                        name: Some("terminal".to_owned()),
                    },
                    kind: LauncherItemKind::Shell,
                    graphical: false,
                    capabilities: vec!["persistent-shell".to_owned()],
                },
            ],
            ..Default::default()
        };
        WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Launcher,
            realm_groups: vec![RealmGroup {
                realm_name: "host".to_owned(),
                realm_id: "host".to_owned(),
                realm_color: "#ff8080".to_owned(),
                workloads: vec![workload],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn daemon_down_blocks_lifecycle() {
        let state = WlState {
            connectivity: Connectivity::DaemonDown,
            ..Default::default()
        };
        let reason = block_reason(
            &ActionKind::Start {
                vm: "corp-vm".into(),
            },
            &state,
        );
        assert!(matches!(reason, Some(Unavailable::DaemonDown)));
    }

    #[test]
    fn terminal_requires_admin() {
        let state = connected_state(
            AuthRole::Launcher,
            vec![vm("corp-vm", RuntimeState::Running)],
        );
        let reason = block_reason(
            &ActionKind::LaunchTerminal {
                vm: "corp-vm".into(),
            },
            &state,
        );
        assert!(matches!(
            reason,
            Some(Unavailable::InsufficientRole {
                required: AuthRole::Admin
            })
        ));
    }

    #[test]
    fn role_gating_distinguishes_lifecycle_and_terminal_privileges() {
        let no_role = connected_state(AuthRole::None, vec![vm("corp-vm", RuntimeState::Stopped)]);
        let lifecycle = ActionKind::Start {
            vm: "corp-vm".into(),
        };
        assert!(matches!(
            plan(&lifecycle, &no_role, &Config::default()),
            Err(Unavailable::InsufficientRole {
                required: AuthRole::Admin
            })
        ));

        let launcher = connected_state(
            AuthRole::Launcher,
            vec![vm("corp-vm", RuntimeState::Stopped)],
        );
        assert!(matches!(
            plan(&lifecycle, &launcher, &Config::default()),
            Err(Unavailable::InsufficientRole {
                required: AuthRole::Admin
            })
        ));
        let build = ActionKind::Build {
            vm: "corp-vm".into(),
        };
        assert!(plan(&build, &launcher, &Config::default()).is_ok());

        let running_launcher = connected_state(
            AuthRole::Launcher,
            vec![vm("corp-vm", RuntimeState::Running)],
        );
        let terminal = ActionKind::LaunchTerminal {
            vm: "corp-vm".into(),
        };
        assert!(matches!(
            plan(&terminal, &running_launcher, &Config::default()),
            Err(Unavailable::InsufficientRole {
                required: AuthRole::Admin
            })
        ));

        let admin = connected_state(AuthRole::Admin, vec![vm("corp-vm", RuntimeState::Running)]);
        assert!(plan(&terminal, &admin, &Config::default()).is_ok());
    }

    #[test]
    fn terminal_argv_has_no_shell_string() {
        let state = connected_state(AuthRole::Admin, vec![vm("corp-vm", RuntimeState::Running)]);
        let config = Config::default();
        let planned = plan(
            &ActionKind::LaunchTerminal {
                vm: "corp-vm".into(),
            },
            &state,
            &config,
        )
        .expect("plannable");
        match planned {
            PlannedAction::Process { argv, wait } => {
                assert!(wait);
                assert_eq!(argv[0], "d2b");
                assert!(argv.contains(&"corp-vm".to_owned()));
                assert!(argv.contains(&"-d".to_owned()));
                assert!(!argv.contains(&"-it".to_owned()));
                assert!(argv.iter().all(|a| !a.contains("&&") && !a.contains("|")));
            }
            other => panic!("expected process, got {other:?}"),
        }
    }

    #[test]
    fn start_blocked_when_already_running() {
        let state = connected_state(AuthRole::Admin, vec![vm("corp-vm", RuntimeState::Running)]);
        let reason = block_reason(
            &ActionKind::Start {
                vm: "corp-vm".into(),
            },
            &state,
        );
        assert!(matches!(reason, Some(Unavailable::VmState { .. })));
    }

    #[test]
    fn running_state_gates_stop_restart_switch_and_terminal() {
        let stopped_admin =
            connected_state(AuthRole::Admin, vec![vm("corp-vm", RuntimeState::Stopped)]);
        for action in [
            ActionKind::Stop {
                vm: "corp-vm".into(),
            },
            ActionKind::ForceStop {
                vm: "corp-vm".into(),
            },
            ActionKind::Restart {
                vm: "corp-vm".into(),
            },
            ActionKind::Switch {
                vm: "corp-vm".into(),
            },
        ] {
            assert!(matches!(
                plan(&action, &stopped_admin, &Config::default()),
                Err(Unavailable::VmState { .. })
            ));
        }

        let terminal = ActionKind::LaunchTerminal {
            vm: "corp-vm".into(),
        };
        assert!(matches!(
            plan(&terminal, &stopped_admin, &Config::default()),
            Err(Unavailable::VmState { .. })
        ));

        let running_launcher = connected_state(
            AuthRole::Launcher,
            vec![vm("corp-vm", RuntimeState::Running)],
        );
        for action in [
            ActionKind::Stop {
                vm: "corp-vm".into(),
            },
            ActionKind::ForceStop {
                vm: "corp-vm".into(),
            },
            ActionKind::Restart {
                vm: "corp-vm".into(),
            },
            ActionKind::Switch {
                vm: "corp-vm".into(),
            },
        ] {
            assert!(matches!(
                plan(&action, &running_launcher, &Config::default()),
                Err(Unavailable::InsufficientRole {
                    required: AuthRole::Admin
                })
            ));
        }

        let running_admin =
            connected_state(AuthRole::Admin, vec![vm("corp-vm", RuntimeState::Running)]);
        assert!(plan(&terminal, &running_admin, &Config::default()).is_ok());
        for action in [
            ActionKind::Stop {
                vm: "corp-vm".into(),
            },
            ActionKind::ForceStop {
                vm: "corp-vm".into(),
            },
            ActionKind::Restart {
                vm: "corp-vm".into(),
            },
            ActionKind::Switch {
                vm: "corp-vm".into(),
            },
        ] {
            assert!(plan(&action, &running_admin, &Config::default()).is_ok());
        }
    }

    #[test]
    fn runtime_capabilities_hide_unsupported_controls() {
        let mut qemu = vm("media-vm", RuntimeState::Running);
        qemu.capabilities.terminal = false;
        qemu.capabilities.store_verify = false;
        qemu.capabilities.switch = false;
        qemu.capabilities.build = false;
        qemu.capabilities.boot = false;
        let state = connected_state(AuthRole::Admin, vec![qemu]);

        for action in [
            ActionKind::LaunchTerminal {
                vm: "media-vm".into(),
            },
            ActionKind::StoreVerify {
                vm: "media-vm".into(),
            },
            ActionKind::Switch {
                vm: "media-vm".into(),
            },
            ActionKind::Build {
                vm: "media-vm".into(),
            },
            ActionKind::Boot {
                vm: "media-vm".into(),
            },
        ] {
            assert!(matches!(
                block_reason(&action, &state),
                Some(Unavailable::Blocked { detail }) if detail == "unsupported by this VM runtime"
            ));
        }

        assert!(block_reason(
            &ActionKind::UsbAttach {
                vm: "media-vm".into(),
                bus_id: "1-2".into(),
            },
            &state
        )
        .is_none());
    }

    #[test]
    fn usb_attach_and_detach_respect_foreign_owner() {
        let mut owner = vm("dev-vm", RuntimeState::Running);
        owner
            .usb
            .push(usb_claim("dev-vm", "1-2", true, Some("dev-vm")));
        let state = connected_state(
            AuthRole::Admin,
            vec![vm("corp-vm", RuntimeState::Running), owner],
        );

        for action in [
            ActionKind::UsbAttach {
                vm: "corp-vm".into(),
                bus_id: "1-2".into(),
            },
            ActionKind::UsbDetach {
                vm: "corp-vm".into(),
                bus_id: "1-2".into(),
            },
        ] {
            assert!(matches!(
                plan(&action, &state, &Config::default()),
                Err(Unavailable::UsbOwnedElsewhere { owner }) if owner == "dev-vm"
            ));
        }

        let detach_owner = ActionKind::UsbDetach {
            vm: "dev-vm".into(),
            bus_id: "1-2".into(),
        };
        assert!(plan(&detach_owner, &state, &Config::default()).is_ok());
    }

    #[test]
    fn audio_actions_require_daemon_connectivity() {
        let state = WlState {
            connectivity: Connectivity::DaemonDown,
            ..Default::default()
        };
        let actions = [
            ActionKind::AudioMic {
                vm: "corp-vm".into(),
                on: true,
            },
            ActionKind::AudioSpeaker {
                vm: "corp-vm".into(),
                on: true,
            },
            ActionKind::AudioOff {
                vm: "corp-vm".into(),
            },
        ];

        for action in actions {
            assert!(matches!(
                block_reason(&action, &state),
                Some(Unavailable::DaemonDown)
            ));
            assert!(matches!(
                plan(&action, &state, &Config::default()),
                Err(Unavailable::DaemonDown)
            ));
        }
    }

    fn audio_state() -> VmAudioState {
        VmAudioState {
            speaker: AudioChannelState {
                level: Some(80),
                muted: false,
            },
            microphone: AudioChannelState {
                level: Some(50),
                muted: true,
            },
            provider_kind: AudioProviderKind::LocalHypervisor,
            enforcement: AudioEnforcementPosture::HostAndGuest,
            error_kind: None,
            remediation: None,
        }
    }

    #[test]
    fn audio_actions_block_on_bad_posture_and_levels() {
        let mut target = vm("corp-vm", RuntimeState::Running);
        target.features.audio = true;
        target.audio = Some(audio_state());
        let state = connected_state(AuthRole::Admin, vec![target.clone()]);

        let too_high = ActionKind::AudioSpeakerVolume {
            vm: "corp-vm".into(),
            level_percent: 101,
        };
        assert!(matches!(
            plan(&too_high, &state, &Config::default()),
            Err(Unavailable::Blocked { detail }) if detail.contains("between 0 and 100")
        ));

        let mut unsupported = target.clone();
        unsupported.audio.as_mut().expect("audio").enforcement =
            AudioEnforcementPosture::Unsupported;
        let unsupported_state = connected_state(AuthRole::Admin, vec![unsupported]);
        assert!(matches!(
            plan(
                &ActionKind::AudioMic {
                    vm: "corp-vm".into(),
                    on: false,
                },
                &unsupported_state,
                &Config::default()
            ),
            Err(Unavailable::Blocked { detail }) if detail.contains("unsupported")
        ));

        let mut errored = target;
        let audio = errored.audio.as_mut().expect("audio");
        audio.error_kind = Some("provider-misconfigured".into());
        audio.remediation = Some("start guestd".into());
        let errored_state = connected_state(AuthRole::Admin, vec![errored]);
        assert!(matches!(
            plan(
                &ActionKind::AudioSpeaker {
                    vm: "corp-vm".into(),
                    on: true,
                },
                &errored_state,
                &Config::default()
            ),
            Err(Unavailable::Blocked { detail })
                if detail.contains("provider-misconfigured") && detail.contains("start guestd")
        ));
    }

    #[test]
    fn audio_actions_plan_to_public_socket_intents() {
        let mut target = vm("corp-vm", RuntimeState::Running);
        target.features.audio = true;
        target.audio = Some(audio_state());
        let state = connected_state(AuthRole::Admin, vec![target]);

        let planned = plan(
            &ActionKind::AudioMic {
                vm: "corp-vm".into(),
                on: false,
            },
            &state,
            &Config::default(),
        )
        .expect("planned");
        assert_eq!(
            planned,
            PlannedAction::Socket {
                intent: SocketIntent::AudioMute {
                    vm: "corp-vm".into(),
                    channel: AudioChannel::Microphone,
                    mute: true,
                }
            }
        );

        let planned = plan(
            &ActionKind::AudioMicGain {
                vm: "corp-vm".into(),
                level_percent: 33,
            },
            &state,
            &Config::default(),
        )
        .expect("planned");
        assert_eq!(
            planned,
            PlannedAction::Socket {
                intent: SocketIntent::AudioSetVolume {
                    vm: "corp-vm".into(),
                    channel: AudioChannel::Microphone,
                    level_percent: 33,
                }
            }
        );
    }

    #[test]
    fn vm_actions_returns_lifecycle_usb_terminal_store_and_audio() {
        let mut target = vm("corp-vm", RuntimeState::Running);
        target.features.audio = true;
        target.audio = Some(audio_state());
        target.usb.push(UsbClaim {
            vm: "corp-vm".into(),
            env: "work".into(),
            bus_id: "1-2".into(),
            bound: false,
            owner_vm: None,
        });
        let state = connected_state(AuthRole::Admin, vec![target]);

        let actions = vm_actions(&state, &Config::default(), "corp-vm");

        assert_eq!(actions.len(), 16);
        assert!(matches!(&actions[0].action, ActionKind::Start { .. }));
        assert!(matches!(&actions[1].action, ActionKind::Stop { .. }));
        assert!(matches!(&actions[2].action, ActionKind::ForceStop { .. }));
        assert!(matches!(&actions[6].action, ActionKind::Build { .. }));
        assert!(matches!(&actions[7].action, ActionKind::Boot { .. }));
        assert!(matches!(&actions[8].action, ActionKind::Switch { .. }));
        assert!(matches!(&actions[9].action, ActionKind::UsbAttach { .. }));
        assert!(matches!(&actions[10].action, ActionKind::UsbDetach { .. }));
        assert!(matches!(&actions[11].action, ActionKind::AudioMic { .. }));
        assert!(actions[11].unavailable.is_none());
        assert!(matches!(
            &actions[12].action,
            ActionKind::AudioSpeaker { .. }
        ));
        assert!(actions[12].unavailable.is_none());
        assert!(matches!(
            &actions[13].action,
            ActionKind::AudioSpeakerVolume { .. }
        ));
        assert!(matches!(
            &actions[14].action,
            ActionKind::AudioMicGain { .. }
        ));
        assert!(matches!(&actions[15].action, ActionKind::AudioOff { .. }));
    }

    #[test]
    fn vm_actions_blocks_terminal_when_config_is_invalid() {
        let state = connected_state(AuthRole::Admin, vec![vm("corp-vm", RuntimeState::Running)]);
        let config = Config {
            terminal: crate::config::TerminalConfig {
                guest_shell: String::new(),
                guest_argv: vec![],
                ..Default::default()
            },
            ..Default::default()
        };

        let actions = vm_actions(&state, &config, "corp-vm");

        let terminal = actions
            .iter()
            .find(|entry| matches!(&entry.action, ActionKind::LaunchTerminal { .. }))
            .expect("terminal action");
        assert!(matches!(
            &terminal.unavailable,
            Some(Unavailable::Blocked { detail }) if detail.contains("terminal.guest_argv")
        ));
    }

    #[test]
    fn build_and_boot_plan_to_process_and_socket() {
        let state = connected_state(AuthRole::Admin, vec![vm("corp-vm", RuntimeState::Running)]);

        let build = plan(
            &ActionKind::Build {
                vm: "corp-vm".into(),
            },
            &connected_state(
                AuthRole::Launcher,
                vec![vm("corp-vm", RuntimeState::Running)],
            ),
            &Config::default(),
        )
        .expect("build plannable");
        assert_eq!(
            build,
            PlannedAction::Process {
                argv: vec!["d2b".into(), "build".into(), "corp-vm".into()],
                wait: true,
            }
        );

        let boot = plan(
            &ActionKind::Boot {
                vm: "corp-vm".into(),
            },
            &state,
            &Config::default(),
        )
        .expect("boot plannable");
        assert_eq!(
            boot,
            PlannedAction::Socket {
                intent: SocketIntent::Boot {
                    vm: "corp-vm".into()
                }
            }
        );
    }

    #[test]
    fn stop_plans_graceful_by_default_and_force_sets_socket_flag() {
        let state = connected_state(AuthRole::Admin, vec![vm("corp-vm", RuntimeState::Running)]);

        let normal = plan(
            &ActionKind::Stop {
                vm: "corp-vm".into(),
            },
            &state,
            &Config::default(),
        )
        .expect("normal stop plannable");
        assert_eq!(
            normal,
            PlannedAction::Socket {
                intent: SocketIntent::VmStop {
                    vm: "corp-vm".into(),
                    force: false
                }
            }
        );

        let force = plan(
            &ActionKind::ForceStop {
                vm: "corp-vm".into(),
            },
            &state,
            &Config::default(),
        )
        .expect("force stop plannable");
        assert_eq!(
            force,
            PlannedAction::Socket {
                intent: SocketIntent::VmStop {
                    vm: "corp-vm".into(),
                    force: true
                }
            }
        );
    }

    #[test]
    fn process_actions_deserialize_without_wait_for_compatibility() {
        let action: PlannedAction =
            serde_json::from_str(r#"{"dispatch":"process","argv":["d2b","build","corp-vm"]}"#)
                .expect("old process action should deserialize");
        assert_eq!(
            action,
            PlannedAction::Process {
                argv: vec!["d2b".into(), "build".into(), "corp-vm".into()],
                wait: false,
            }
        );
    }

    #[test]
    fn observability_plan_opens_configured_url_without_daemon_state() {
        let planned = plan(
            &ActionKind::OpenObservability,
            &WlState::default(),
            &Config::default(),
        )
        .expect("observability plannable");

        assert_eq!(
            planned,
            PlannedAction::Process {
                argv: vec!["xdg-open".into(), "http://sys-obs:8080".into()],
                wait: false,
            }
        );
    }

    #[test]
    fn quick_launch_uses_configured_detached_guest_argv() {
        let state = connected_state(AuthRole::Admin, vec![vm("work-ssd", RuntimeState::Running)]);
        let config = Config {
            quick_launch: vec![crate::config::QuickLaunchConfig {
                id: "run-openterface".into(),
                vm: "work-ssd".into(),
                icon: "desktop_windows".into(),
                tooltip: "Run Openterface".into(),
                guest_argv: vec!["/run/current-system/sw/bin/openterface-run".into()],
            }],
            ..Default::default()
        };

        let planned = plan(
            &ActionKind::QuickLaunch {
                vm: "work-ssd".into(),
                id: "run-openterface".into(),
            },
            &state,
            &config,
        )
        .expect("quick launch plannable");

        assert_eq!(
            planned,
            PlannedAction::Process {
                argv: vec![
                    "d2b".into(),
                    "vm".into(),
                    "exec".into(),
                    "-d".into(),
                    "work-ssd".into(),
                    "--".into(),
                    "/run/current-system/sw/bin/openterface-run".into()
                ],
                wait: true,
            }
        );
    }

    #[test]
    fn configured_exec_uses_exact_d2b_launch_argv() {
        let state = workload_state(
            WorkloadProviderKind::UnsafeLocal,
            WorkloadAvailability::Ready,
        );
        let planned = plan(
            &ActionKind::WorkloadLaunch {
                target: "tools.host.d2b".to_owned(),
                item_id: "firefox".to_owned(),
                item_kind: LauncherItemKind::Exec,
            },
            &state,
            &Config::default(),
        )
        .expect("configured exec launch");
        assert_eq!(
            planned,
            PlannedAction::Process {
                argv: vec![
                    "d2b".to_owned(),
                    "launch".to_owned(),
                    "tools.host.d2b".to_owned(),
                    "--item".to_owned(),
                    "firefox".to_owned(),
                ],
                wait: true,
            }
        );
    }

    #[test]
    fn shell_item_routes_to_wlterm_with_canonical_target() {
        let state = workload_state(
            WorkloadProviderKind::UnsafeLocal,
            WorkloadAvailability::Ready,
        );
        let planned = plan(
            &ActionKind::WorkloadLaunch {
                target: "tools.host.d2b".to_owned(),
                item_id: "terminal".to_owned(),
                item_kind: LauncherItemKind::Shell,
            },
            &state,
            &Config::default(),
        )
        .expect("persistent shell launch");
        assert_eq!(
            planned,
            PlannedAction::Process {
                argv: vec![
                    "d2b-wlterm".to_owned(),
                    "open".to_owned(),
                    "tools.host.d2b".to_owned(),
                    "terminal".to_owned(),
                ],
                wait: true,
            }
        );
    }

    #[test]
    fn helper_unavailable_blocks_launch_with_remediation() {
        let state = workload_state(
            WorkloadProviderKind::UnsafeLocal,
            WorkloadAvailability::HelperUnavailable,
        );
        let error = plan(
            &ActionKind::WorkloadLaunch {
                target: "tools.host.d2b".to_owned(),
                item_id: "firefox".to_owned(),
                item_kind: LauncherItemKind::Exec,
            },
            &state,
            &Config::default(),
        )
        .expect_err("helper unavailable");
        assert!(matches!(error, Unavailable::Blocked { detail }
                if detail.contains("enable and start") && detail.contains("user service")));
    }

    #[test]
    fn unsafe_local_vm_shaped_controls_fail_closed() {
        let mut state = workload_state(
            WorkloadProviderKind::UnsafeLocal,
            WorkloadAvailability::Ready,
        );
        state.role = AuthRole::Admin;
        state.vms.push(Vm {
            name: "tools".to_owned(),
            canonical_target: Some("tools.host.d2b".to_owned()),
            state: RuntimeState::Running,
            ..Default::default()
        });
        for action in [
            ActionKind::Stop {
                vm: "tools".to_owned(),
            },
            ActionKind::Build {
                vm: "tools".to_owned(),
            },
            ActionKind::StoreVerify {
                vm: "tools".to_owned(),
            },
            ActionKind::UsbAttach {
                vm: "tools".to_owned(),
                bus_id: "1-2".to_owned(),
            },
            ActionKind::AudioOff {
                vm: "tools".to_owned(),
            },
            ActionKind::LaunchTerminal {
                vm: "tools".to_owned(),
            },
        ] {
            assert!(
                matches!(
                    block_reason(&action, &state),
                    Some(Unavailable::Blocked { detail })
                        if detail.contains("launcher items only")
                ),
                "{action:?}"
            );
        }
    }
}
