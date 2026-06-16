//! Action availability gating and argv/intent planning.
//!
//! Owning wave: **Wave 1 — Core model agent**. Wave 0 ships a working baseline
//! covering role gating, daemon-down gating, and argv-only terminal planning.
//! The Wave 1 agent extends per-action VM-state rules, USB ownership rules, and
//! the full advanced-controls matrix from the plan.

use crate::config::Config;
use crate::model::{
    ActionKind, AuthRole, Connectivity, PlannedAction, RuntimeState, SocketIntent, Unavailable, Vm,
    WlState,
};

/// Returns `Some(reason)` when `action` cannot currently be invoked.
pub fn block_reason(action: &ActionKind, state: &WlState) -> Option<Unavailable> {
    // Display-only actions are always available.
    match action {
        ActionKind::OpenControlCenter | ActionKind::CycleDisplay | ActionKind::Refresh => {
            return None;
        }
        _ => {}
    }

    if state.connectivity == Connectivity::DaemonDown {
        return Some(Unavailable::DaemonDown);
    }

    let required = required_role(action);
    if !role_satisfies(state.role, required) {
        return Some(Unavailable::InsufficientRole { required });
    }

    match action {
        ActionKind::Start { vm } => running_vm(state, vm)
            .filter(|v| v.state == RuntimeState::Running)
            .map(|_| Unavailable::VmState {
                detail: "VM is already running".into(),
            }),
        ActionKind::Stop { vm } | ActionKind::Restart { vm } | ActionKind::Switch { vm } => {
            running_vm(state, vm)
                .filter(|v| v.state == RuntimeState::Stopped)
                .map(|_| Unavailable::VmState {
                    detail: "VM is not running".into(),
                })
        }
        ActionKind::LaunchTerminal { vm } => running_vm(state, vm)
            .filter(|v| v.state != RuntimeState::Running)
            .map(|_| Unavailable::VmState {
                detail: "start the VM before opening a terminal".into(),
            }),
        ActionKind::UsbAttach { vm, bus_id } => usb_attach_block(state, vm, bus_id),
        ActionKind::UsbDetach { vm, bus_id } => usb_detach_block(state, vm, bus_id),
        ActionKind::StoreVerify { .. } => None,
        _ => None,
    }
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

    let dispatch = match action {
        ActionKind::Start { vm } => socket(SocketIntent::VmStart { vm: vm.clone() }),
        ActionKind::Stop { vm } => socket(SocketIntent::VmStop { vm: vm.clone() }),
        ActionKind::Restart { vm } => socket(SocketIntent::VmRestart { vm: vm.clone() }),
        ActionKind::Switch { vm } => socket(SocketIntent::Switch { vm: vm.clone() }),
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
        ActionKind::OpenControlCenter | ActionKind::CycleDisplay => {
            // These are handled in-process by the UI/Waybar layers, not as a
            // nixling dispatch; planning them is a no-op socket refresh.
            return Err(Unavailable::Blocked {
                detail: "handled in-process; not a nixling dispatch".into(),
            });
        }
    };
    Ok(dispatch)
}

fn required_role(action: &ActionKind) -> AuthRole {
    match action {
        ActionKind::LaunchTerminal { .. } => AuthRole::Admin,
        ActionKind::Start { .. }
        | ActionKind::Stop { .. }
        | ActionKind::Restart { .. }
        | ActionKind::Switch { .. }
        | ActionKind::UsbAttach { .. }
        | ActionKind::UsbDetach { .. }
        | ActionKind::StoreVerify { .. } => AuthRole::Launcher,
        _ => AuthRole::None,
    }
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

/// Build the argv-only terminal launch command. There is no shell string and no
/// interpolation: the terminal argv prefix, the nixling exec invocation, and the
/// guest shell are concatenated as discrete argv elements.
fn terminal_argv(vm: &str, config: &Config) -> PlannedAction {
    let mut argv = config.terminal.argv.clone();
    argv.extend([
        "nixling".to_owned(),
        "vm".to_owned(),
        "exec".to_owned(),
        "-it".to_owned(),
        vm.to_owned(),
        "--".to_owned(),
        config.terminal.guest_shell.clone(),
    ]);
    PlannedAction::Process { argv }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::VmFeatures;

    fn connected_state(role: AuthRole, vms: Vec<Vm>) -> WlState {
        WlState {
            connectivity: Connectivity::Connected,
            role,
            vms,
            stale: false,
            note: None,
        }
    }

    fn vm(name: &str, state: RuntimeState) -> Vm {
        Vm {
            name: name.into(),
            env: None,
            state,
            is_net_vm: false,
            pending_restart: false,
            features: VmFeatures::default(),
            static_ip: None,
            readiness: vec![],
            usb: vec![],
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
            PlannedAction::Process { argv } => {
                assert_eq!(argv[0], "foot");
                assert!(argv.contains(&"corp-vm".to_owned()));
                assert!(argv.iter().all(|a| !a.contains("&&") && !a.contains("|")));
            }
            other => panic!("expected process, got {other:?}"),
        }
    }

    #[test]
    fn start_blocked_when_already_running() {
        let state = connected_state(
            AuthRole::Launcher,
            vec![vm("corp-vm", RuntimeState::Running)],
        );
        let reason = block_reason(
            &ActionKind::Start {
                vm: "corp-vm".into(),
            },
            &state,
        );
        assert!(matches!(reason, Some(Unavailable::VmState { .. })));
    }
}
