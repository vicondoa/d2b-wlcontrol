//! Waybar custom-module rendering.
//!
//! Waybar custom-module contract (`return-type = "json"`): one newline-
//! terminated JSON object per update with `text`, `class`, and `tooltip`.

use serde::{Deserialize, Serialize};
use wlcontrol_core::model::{AuthRole, Connectivity, RealmGroup, RuntimeState, Vm, WlState};

use d2b_wayland_waybar::WaybarModule;

const DETAIL_VM_LIMIT: usize = 5;

/// Text density for the Waybar module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DisplayMode {
    Compact,
    Detail,
}

/// A single Waybar JSON line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WaybarLine {
    pub text: String,
    /// CSS classes (Waybar accepts an array of classes).
    pub class: Vec<String>,
    pub tooltip: String,
}

impl WaybarLine {
    /// Serialize to a single newline-terminated JSON line, as Waybar expects.
    pub fn to_json_line(&self) -> String {
        let module = WaybarModule {
            text: self.text.clone(),
            alt: None,
            tooltip: Some(self.tooltip.clone()),
            class: self.class.clone(),
            percentage: None,
        };
        let mut s = module.to_waybar_json().unwrap_or_else(|_| {
            "{\"text\":\"\",\"class\":[\"error\"],\"tooltip\":\"render error\"}".to_owned()
        });
        s.push('\n');
        s
    }
}

/// Render the compact Waybar line for a reduced state.
pub fn render(state: &WlState) -> WaybarLine {
    render_mode(state, DisplayMode::Compact)
}

/// Render the Waybar line for the requested display mode.
pub fn render_mode(state: &WlState, mode: DisplayMode) -> WaybarLine {
    render_mode_with_presentation(state, mode, "◆", "")
}

/// Render with a bounded user-configured icon and label.
pub fn render_mode_with_presentation(
    state: &WlState,
    mode: DisplayMode,
    icon: &str,
    label: &str,
) -> WaybarLine {
    let prefix = presentation_prefix(icon, label);
    match state.connectivity {
        Connectivity::DaemonDown => WaybarLine {
            text: format!("{prefix} down"),
            class: state_classes(state),
            tooltip: render_tooltip(state),
        },
        Connectivity::AuthDenied => WaybarLine {
            text: format!("{prefix} auth"),
            class: state_classes(state),
            tooltip: render_tooltip(state),
        },
        Connectivity::Connected => render_connected(state, mode, &prefix),
    }
}

fn presentation_prefix(icon: &str, label: &str) -> String {
    let icon = icon
        .chars()
        .filter(|ch| !ch.is_control())
        .take(8)
        .collect::<String>();
    let label = label
        .chars()
        .filter(|ch| !ch.is_control())
        .take(32)
        .collect::<String>();
    match (icon.trim(), label.trim()) {
        ("", "") => "◆".to_owned(),
        ("", label) => label.to_owned(),
        (icon, "") => icon.to_owned(),
        (icon, label) => format!("{icon} {label}"),
    }
}

fn render_connected(state: &WlState, mode: DisplayMode, prefix: &str) -> WaybarLine {
    let running = state.running_count();
    let total = state.visible_count();
    let attention = state.needs_attention();

    let mut text = compact_text(prefix, running, total, attention);
    if mode == DisplayMode::Detail {
        text = detail_text(state, &text);
    }

    WaybarLine {
        text,
        class: state_classes(state),
        tooltip: render_tooltip(state),
    }
}

fn compact_text(prefix: &str, running: usize, total: usize, attention: bool) -> String {
    let mut text = format!("{prefix} {running}/{total}");
    if attention {
        text.push_str(" !");
    }
    text
}

fn detail_text(state: &WlState, prefix: &str) -> String {
    let visible = visible_vms(state).collect::<Vec<_>>();
    if visible.is_empty() {
        return prefix.to_owned();
    }

    let mut parts = visible
        .iter()
        .take(DETAIL_VM_LIMIT)
        .map(|vm| format!("{}:{}", vm.name, state_glyph(vm.state)))
        .collect::<Vec<_>>();
    if visible.len() > DETAIL_VM_LIMIT {
        parts.push(format!("+{}", visible.len() - DETAIL_VM_LIMIT));
    }

    format!("{prefix} · {}", parts.join(" "))
}

fn state_classes(state: &WlState) -> Vec<String> {
    let mut class = match state.connectivity {
        Connectivity::DaemonDown => vec!["daemon-down".to_owned()],
        Connectivity::AuthDenied => vec!["auth-denied".to_owned()],
        Connectivity::Connected => {
            let running = state.running_count();
            let total = state.visible_count();
            vec![match (running, total) {
                (0, _) => "all-stopped".to_owned(),
                (r, t) if r == t => "all-running".to_owned(),
                _ => "partial-running".to_owned(),
            }]
        }
    };
    if state.needs_attention() {
        class.push("attention".to_owned());
    }
    if state.stale {
        class.push("stale".to_owned());
    }
    if state.realm_groups.iter().any(|group| {
        group
            .workloads
            .iter()
            .any(|workload| workload.is_unsafe_local())
    }) {
        class.push("unsafe-local".to_owned());
    }
    if state
        .realm_groups
        .iter()
        .any(RealmGroup::has_mixed_isolation)
    {
        class.push("mixed-isolation".to_owned());
    }
    class
}

fn render_tooltip(state: &WlState) -> String {
    let mut lines = vec![format!(
        "role: {} · connectivity: {}",
        role_label(state.role),
        connectivity_label(state.connectivity)
    )];
    if let Some(note) = &state.note {
        lines.push(format!("note: {note}"));
    }

    let mut any = false;
    for vm in visible_vms(state) {
        any = true;
        lines.push(vm_tooltip_line(vm));
    }
    if !any {
        lines.push("No visible VMs".to_owned());
    }

    // Realm-grouped quick-launch section.
    let realm_section = render_realm_quick_launch_section(&state.realm_groups);
    if !realm_section.is_empty() {
        lines.push(String::new()); // blank separator
        lines.push(realm_section);
    }

    lines.join("\n")
}

/// Render the realm quick-launch panel section as a tooltip block.
///
/// Returns an empty string when there are no realm groups.
pub fn render_realm_quick_launch_section(groups: &[RealmGroup]) -> String {
    if groups.is_empty() {
        return String::new();
    }
    let mut lines: Vec<String> = Vec::new();
    lines.push("— realm launchers —".to_owned());
    for group in groups {
        if group.workloads.is_empty() {
            continue;
        }
        let warning = if group.all_unsafe_local() {
            " · unsafe-local: no isolation, user-manager lifetime"
        } else {
            ""
        };
        lines.push(format!(
            "{} ({}){warning}",
            group.realm_name, group.realm_color
        ));
        for workload in &group.workloads {
            let items = workload
                .launcher_items
                .iter()
                .map(|item| format!("[{} {}]", item.icon.preferred(), item.name))
                .collect::<Vec<_>>();
            let row_warning = if group.has_mixed_isolation() && workload.is_unsafe_local() {
                " · unsafe-local: no isolation, user-manager lifetime"
            } else {
                ""
            };
            let provider = if group.all_unsafe_local() {
                String::new()
            } else {
                format!(" [{}]", provider_label(workload.provider_kind))
            };
            lines.push(format!(
                "  {}{provider}{row_warning}: {}",
                workload.label,
                items.join(" · ")
            ));
            if let Some(remediation) = workload.availability.remediation().filter(|_| {
                workload.availability != wlcontrol_core::model::WorkloadAvailability::Ready
            }) {
                lines.push(format!("    {remediation}"));
            }
        }
    }
    lines.join("\n")
}

fn provider_label(kind: wlcontrol_core::model::WorkloadProviderKind) -> &'static str {
    use wlcontrol_core::model::WorkloadProviderKind;
    match kind {
        WorkloadProviderKind::LocalVm => "local-vm",
        WorkloadProviderKind::QemuMedia => "qemu-media",
        WorkloadProviderKind::ProviderManaged => "provider-managed",
        WorkloadProviderKind::UnsafeLocal => "unsafe-local",
        WorkloadProviderKind::Unknown => "unknown",
    }
}

fn visible_vms(state: &WlState) -> impl Iterator<Item = &Vm> {
    state.vms.iter().filter(|v| !v.is_net_vm && !v.hidden)
}

fn vm_tooltip_line(vm: &Vm) -> String {
    let env = vm.env.as_deref().unwrap_or("-");
    let mut line = format!(
        "{} {} · env={} · state={}",
        state_glyph(vm.state),
        vm.name,
        env,
        state_label(vm.state)
    );
    if vm.pending_restart {
        line.push_str(" · pending-restart");
    }

    let usb = vm
        .usb
        .iter()
        .filter(|claim| claim.bound)
        .map(|claim| {
            let owner = claim.owner_vm.as_deref().unwrap_or("unknown");
            format!("{}→{}", claim.bus_id, owner)
        })
        .collect::<Vec<_>>();
    if !usb.is_empty() {
        line.push_str(" · usb=");
        line.push_str(&usb.join(","));
    }
    if let Some(audio) = &vm.audio {
        if let Some(kind) = &audio.error_kind {
            line.push_str(" · audio=");
            line.push_str(kind);
            if let Some(remediation) = &audio.remediation {
                line.push_str(" (");
                line.push_str(remediation);
                line.push(')');
            }
        } else {
            line.push_str(" · audio=");
            line.push_str(if audio.speaker.muted {
                "spk-off"
            } else {
                "spk-on"
            });
            if let Some(level) = audio.speaker.level {
                line.push('(');
                line.push_str(&level.to_string());
                line.push_str("%)");
            }
            line.push('/');
            line.push_str(if audio.microphone.muted {
                "mic-off"
            } else {
                "mic-on!"
            });
            if let Some(level) = audio.microphone.level {
                line.push('(');
                line.push_str(&level.to_string());
                line.push_str("%)");
            }
            line.push(' ');
            line.push_str(match audio.enforcement {
                wlcontrol_core::model::AudioEnforcementPosture::HostAndGuest => "host+guest",
                wlcontrol_core::model::AudioEnforcementPosture::HostOnly => "host-only",
                wlcontrol_core::model::AudioEnforcementPosture::GuestOnly => "guest-only",
                wlcontrol_core::model::AudioEnforcementPosture::Unsupported => "unsupported",
                wlcontrol_core::model::AudioEnforcementPosture::Unknown => "unknown",
            });
        }
    }

    line
}

fn state_glyph(state: RuntimeState) -> &'static str {
    match state {
        RuntimeState::Running => "●",
        RuntimeState::Starting | RuntimeState::Stopping => "◐",
        RuntimeState::Stopped => "○",
        RuntimeState::Unknown => "?",
    }
}

fn state_label(state: RuntimeState) -> &'static str {
    match state {
        RuntimeState::Running => "running",
        RuntimeState::Starting => "starting",
        RuntimeState::Stopping => "stopping",
        RuntimeState::Stopped => "stopped",
        RuntimeState::Unknown => "unknown",
    }
}

fn connectivity_label(connectivity: Connectivity) -> &'static str {
    match connectivity {
        Connectivity::Connected => "connected",
        Connectivity::AuthDenied => "auth-denied",
        Connectivity::DaemonDown => "daemon-down",
    }
}

fn role_label(role: AuthRole) -> &'static str {
    match role {
        AuthRole::None => "none",
        AuthRole::Launcher => "launcher",
        AuthRole::Admin => "admin",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wlcontrol_core::model::{
        AudioChannelState, AudioEnforcementPosture, AudioProviderKind, LauncherIcon,
        LauncherItemKind, LauncherItemSummary, RealmLauncherEntry, UsbClaim, VmAudioState,
        VmFeatures, WorkloadAvailability, WorkloadProviderKind,
    };

    fn vm(name: &str, state: RuntimeState, net: bool) -> Vm {
        Vm {
            name: name.into(),
            canonical_target: None,
            env: Some("work".into()),
            state,
            is_net_vm: net,
            hidden: false,
            pending_restart: false,
            features: VmFeatures::default(),
            capabilities: Default::default(),
            static_ip: None,
            readiness: vec![],
            usb: vec![],
            audio: None,
            quick_launch: vec![],
        }
    }

    fn realm_entry(action_id: &str, icon: &str, collision: bool) -> RealmLauncherEntry {
        RealmLauncherEntry {
            action_id: action_id.to_owned(),
            workload_name: action_id.to_owned(),
            label: action_id.to_owned(),
            icon: icon.to_owned(),
            canonical_target: format!("{action_id}.work.d2b"),
            legacy_vm_name: Some(action_id.to_owned()),
            has_icon_collision: collision,
            icon_siblings: if collision {
                vec!["other".to_owned()]
            } else {
                vec![]
            },
            provider_kind: WorkloadProviderKind::LocalVm,
            availability: WorkloadAvailability::Ready,
            launcher_items: vec![LauncherItemSummary {
                id: action_id.to_owned(),
                name: action_id.to_owned(),
                icon: LauncherIcon {
                    id: Some(icon.to_owned()),
                    name: None,
                },
                kind: LauncherItemKind::Exec,
                graphical: true,
                capabilities: vec!["configured-launch".to_owned()],
            }],
            ..Default::default()
        }
    }

    fn usb_claim(bus_id: &str, owner_vm: Option<&str>) -> UsbClaim {
        UsbClaim {
            vm: "corp-vm".into(),
            env: "work".into(),
            bus_id: bus_id.into(),
            bound: true,
            owner_vm: owner_vm.map(str::to_owned),
        }
    }

    #[test]
    fn daemon_down_line() {
        let state = WlState {
            connectivity: Connectivity::DaemonDown,
            ..Default::default()
        };
        let line = render(&state);
        assert!(line.class.contains(&"daemon-down".to_owned()));
        assert!(line.to_json_line().ends_with("\n"));
        assert!(line.tooltip.contains("connectivity: daemon-down"));
    }

    #[test]
    fn partial_running_excludes_net_vm() {
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: wlcontrol_core::model::AuthRole::Admin,
            vms: vec![
                vm("a", RuntimeState::Running, false),
                vm("b", RuntimeState::Stopped, false),
                vm("sys-work-net", RuntimeState::Running, true),
            ],
            stale: false,
            note: None,
            ..Default::default()
        };
        let line = render(&state);
        assert_eq!(line.text, "◆ 1/2");
        assert!(line.class.contains(&"partial-running".to_owned()));
        assert!(!line.text.contains("sys-work-net"));
    }

    #[test]
    fn render_delegates_to_compact_mode() {
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Admin,
            vms: vec![vm("a", RuntimeState::Running, false)],
            stale: false,
            note: None,
            ..Default::default()
        };
        assert_eq!(render(&state), render_mode(&state, DisplayMode::Compact));
    }

    #[test]
    fn compact_and_detail_text_are_distinct() {
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Admin,
            vms: vec![
                vm("a", RuntimeState::Running, false),
                vm("b", RuntimeState::Stopped, false),
            ],
            stale: false,
            note: None,
            ..Default::default()
        };

        let compact = render_mode(&state, DisplayMode::Compact);
        let detail = render_mode(&state, DisplayMode::Detail);

        assert_eq!(compact.text, "◆ 1/2");
        assert_eq!(detail.text, "◆ 1/2 · a:● b:○");
    }

    #[test]
    fn detail_mode_caps_visible_vm_count() {
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Launcher,
            vms: (0..7)
                .map(|n| vm(&format!("vm{n}"), RuntimeState::Stopped, false))
                .collect(),
            stale: false,
            note: None,
            ..Default::default()
        };

        let detail = render_mode(&state, DisplayMode::Detail);

        assert_eq!(detail.text, "◆ 0/7 · vm0:○ vm1:○ vm2:○ vm3:○ vm4:○ +2");
        assert!(!detail.text.contains("vm5:"));
    }

    #[test]
    fn classes_include_attention_and_stale() {
        let mut target = vm("a", RuntimeState::Running, false);
        target.pending_restart = true;
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Admin,
            vms: vec![target],
            stale: true,
            note: None,
            ..Default::default()
        };

        let line = render(&state);

        assert!(line.class.contains(&"all-running".to_owned()));
        assert!(line.class.contains(&"attention".to_owned()));
        assert!(line.class.contains(&"stale".to_owned()));
        assert_eq!(line.text, "◆ 1/1 !");
    }

    #[test]
    fn audio_microphone_and_errors_trigger_attention_tooltip_detail() {
        let mut target = vm("corp-vm", RuntimeState::Running, false);
        target.audio = Some(VmAudioState {
            speaker: AudioChannelState {
                level: Some(80),
                muted: false,
            },
            microphone: AudioChannelState {
                level: Some(50),
                muted: false,
            },
            provider_kind: AudioProviderKind::LocalHypervisor,
            enforcement: AudioEnforcementPosture::HostAndGuest,
            error_kind: None,
            remediation: None,
        });
        let mut errored = vm("aca-vm", RuntimeState::Stopped, false);
        errored.audio = Some(VmAudioState {
            speaker: AudioChannelState {
                level: None,
                muted: true,
            },
            microphone: AudioChannelState {
                level: None,
                muted: true,
            },
            provider_kind: AudioProviderKind::Unknown,
            enforcement: AudioEnforcementPosture::Unsupported,
            error_kind: Some("provider-misconfigured".into()),
            remediation: Some("start guestd".into()),
        });
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Admin,
            vms: vec![target, errored],
            stale: false,
            note: None,
            ..Default::default()
        };

        let line = render(&state);

        assert!(line.class.contains(&"attention".to_owned()));
        assert!(line.tooltip.contains("audio=spk-on(80%)/mic-on!(50%)"));
        assert!(line
            .tooltip
            .contains("audio=provider-misconfigured (start guestd)"));
    }

    #[test]
    fn class_matrix_covers_auth_denied_and_all_stopped() {
        let auth_denied = WlState {
            connectivity: Connectivity::AuthDenied,
            ..Default::default()
        };
        let auth_line = render(&auth_denied);
        assert_eq!(auth_line.text, "◆ auth");
        assert!(auth_line.class.contains(&"auth-denied".to_owned()));

        let all_stopped = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Admin,
            vms: vec![
                vm("a", RuntimeState::Stopped, false),
                vm("b", RuntimeState::Stopped, false),
            ],
            stale: false,
            note: None,
            ..Default::default()
        };
        let stopped_line = render(&all_stopped);
        assert_eq!(stopped_line.text, "◆ 0/2");
        assert!(stopped_line.class.contains(&"all-stopped".to_owned()));
    }

    #[test]
    fn tooltip_includes_role_vm_state_pending_restart_and_usb_owner() {
        let mut target = vm("corp-vm", RuntimeState::Unknown, false);
        target.pending_restart = true;
        target.usb.push(usb_claim("1-2", Some("corp-vm")));
        let mut hidden = vm("hidden-vm", RuntimeState::Running, false);
        hidden.hidden = true;
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Admin,
            vms: vec![
                target,
                hidden,
                vm("sys-work-net", RuntimeState::Running, true),
            ],
            stale: false,
            note: Some("cached after refresh failure".to_owned()),
            ..Default::default()
        };

        let line = render(&state);

        assert!(line
            .tooltip
            .contains("role: admin · connectivity: connected"));
        assert!(line.tooltip.contains("note: cached after refresh failure"));
        assert!(line
            .tooltip
            .contains("? corp-vm · env=work · state=unknown"));
        assert!(line.tooltip.contains("pending-restart"));
        assert!(line.tooltip.contains("usb=1-2→corp-vm"));
        assert!(!line.tooltip.contains("hidden-vm"));
        assert!(!line.tooltip.contains("sys-work-net"));
    }

    #[test]
    fn json_line_is_single_line() {
        let line = render(&WlState::default());
        let json = line.to_json_line();
        assert_eq!(json.matches('\n').count(), 1);
        assert!(json.ends_with('\n'));
    }

    #[test]
    fn json_line_uses_toolkit_waybar_shape_without_changing_fields() {
        let line = WaybarLine {
            text: "◆ 1/1".to_owned(),
            class: vec!["all-running".to_owned()],
            tooltip: "ready".to_owned(),
        };
        let value: serde_json::Value = serde_json::from_str(line.to_json_line().trim()).unwrap();
        assert_eq!(value["text"], "◆ 1/1");
        assert_eq!(value["tooltip"], "ready");
        assert_eq!(value["class"], serde_json::json!(["all-running"]));
        assert!(value.get("alt").is_none());
        assert!(value.get("percentage").is_none());
    }

    #[test]
    fn realm_quick_launch_section_empty_when_no_groups() {
        assert!(render_realm_quick_launch_section(&[]).is_empty());
    }

    #[test]
    fn realm_quick_launch_section_renders_groups_and_icons() {
        let groups = vec![
            RealmGroup {
                realm_name: "work".to_owned(),
                realm_id: "work".to_owned(),
                realm_color: "#90d090".to_owned(),
                workloads: vec![
                    realm_entry("browser", "web", false),
                    realm_entry("terminal", "terminal", false),
                ],
            },
            RealmGroup {
                realm_name: "personal".to_owned(),
                realm_id: "personal".to_owned(),
                realm_color: "#7fc8ff".to_owned(),
                workloads: vec![realm_entry("music", "music_note", false)],
            },
        ];
        let section = render_realm_quick_launch_section(&groups);
        assert!(section.contains("— realm launchers —"));
        assert!(section.contains("work (#90d090)"));
        assert!(section.contains("[web browser]"));
        assert!(section.contains("[terminal terminal]"));
        assert!(section.contains("personal (#7fc8ff)"));
        assert!(section.contains("[music_note music]"));
    }

    #[test]
    fn realm_quick_launch_section_renders_generic_item_names_despite_icon_collisions() {
        let groups = vec![RealmGroup {
            realm_name: "work".to_owned(),
            realm_id: "work".to_owned(),
            realm_color: "#ff8000".to_owned(),
            workloads: vec![
                realm_entry("browser", "web", true),
                realm_entry("mail", "web", true),
                realm_entry("terminal", "terminal", false),
            ],
        }];
        let section = render_realm_quick_launch_section(&groups);
        assert!(section.contains("[web browser]"), "section: {section}");
        assert!(section.contains("[web mail]"), "section: {section}");
        assert!(
            section.contains("[terminal terminal]"),
            "section: {section}"
        );
    }

    #[test]
    fn tooltip_includes_realm_section_when_groups_present() {
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Admin,
            vms: vec![vm("corp-vm", RuntimeState::Running, false)],
            realm_groups: vec![RealmGroup {
                realm_name: "work".to_owned(),
                realm_id: "work".to_owned(),
                realm_color: "#90d090".to_owned(),
                workloads: vec![realm_entry("browser", "web", false)],
            }],
            stale: false,
            note: None,
        };
        let line = render(&state);
        assert!(line.tooltip.contains("— realm launchers —"));
        assert!(line.tooltip.contains("work (#90d090)"));
        assert!(line.tooltip.contains("[web browser]"));
    }

    #[test]
    fn tooltip_omits_realm_section_when_no_groups() {
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Admin,
            vms: vec![vm("corp-vm", RuntimeState::Running, false)],
            ..Default::default()
        };
        let line = render(&state);
        assert!(!line.tooltip.contains("— realm launchers —"));
    }

    #[test]
    fn unsafe_local_uses_only_bounded_classes_and_actionable_tooltip() {
        let mut entry = realm_entry("firefox", "web-browser", false);
        entry.provider_kind = WorkloadProviderKind::UnsafeLocal;
        entry.availability = WorkloadAvailability::HelperUnavailable;
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Launcher,
            realm_groups: vec![RealmGroup {
                realm_name: "host".to_owned(),
                realm_id: "host".to_owned(),
                realm_color: "#ff8080".to_owned(),
                workloads: vec![entry],
            }],
            ..Default::default()
        };
        let line = render(&state);
        assert!(line.class.contains(&"unsafe-local".to_owned()));
        assert!(line.class.contains(&"attention".to_owned()));
        assert!(!line.class.iter().any(|class| class.contains("firefox")));
        assert!(!line.class.iter().any(|class| class.contains("host")));
        assert!(line.tooltip.contains("no isolation"));
        assert!(line.tooltip.contains("enable and start"));
        assert_eq!(
            line.tooltip.matches("unsafe-local: no isolation").count(),
            1
        );
    }

    #[test]
    fn presentation_icon_and_label_are_bounded_without_changing_json_shape() {
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Launcher,
            ..Default::default()
        };
        let line = render_mode_with_presentation(
            &state,
            DisplayMode::Compact,
            "◆◆◆◆◆◆◆◆◆◆",
            "desktop-control-label-that-is-intentionally-too-long",
        );
        assert!(line.text.chars().count() < 50);
        assert!(line.to_json_line().ends_with('\n'));
    }

    #[test]
    fn unsafe_local_waybar_output_matches_golden() {
        let mut entry = realm_entry("tools", "web-browser", false);
        entry.label = "Host Tools".to_owned();
        entry.provider_kind = WorkloadProviderKind::UnsafeLocal;
        entry.availability = WorkloadAvailability::HelperUnavailable;
        entry.launcher_items[0].name = "Firefox".to_owned();
        entry.launcher_items[0].icon.name = Some("web-browser".to_owned());
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Launcher,
            realm_groups: vec![RealmGroup {
                realm_name: "host".to_owned(),
                realm_id: "host".to_owned(),
                realm_color: "#ff8080".to_owned(),
                workloads: vec![entry],
            }],
            ..Default::default()
        };
        let actual: serde_json::Value =
            serde_json::from_str(render(&state).to_json_line().trim()).expect("Waybar JSON");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/golden/unsafe-local-waybar.json"
        ))
        .expect("golden Waybar JSON");
        assert_eq!(actual, expected);
    }
}
