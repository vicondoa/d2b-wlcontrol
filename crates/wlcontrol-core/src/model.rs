//! Frozen cross-crate domain contract for d2b-wlcontrol.
//!
//! These types are the **stable internal contract** that every other crate
//! builds against:
//!
//! - `wlcontrol-d2b` produces [`WlState`] / [`Vm`] / [`UsbClaim`] from the
//!   d2bd public socket.
//! - `wlcontrol-waybar` and `wlcontrol-ui` render [`WlState`].
//! - `wlcontrol-cli` dispatches [`PlannedAction`].
//!
//! Downstream crates may extend these types additively but must not break
//! published field or variant names.

use serde::{Deserialize, Serialize};

/// Effective operator authorization, mirrored from `d2b auth status`.
///
/// This gates which controls the UI may enable. `Admin` is required for
/// guest-control exec (terminal launch); lifecycle/USB verbs require at least
/// the launcher role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AuthRole {
    /// No recognized role; the public socket is unreachable or denied.
    #[default]
    None,
    /// Recognized launcher. Current d2bd keeps destructive lifecycle/USB
    /// verbs admin-only; wlcontrol uses this role for non-destructive build.
    Launcher,
    /// Full admin: launcher plus guest-control exec.
    Admin,
}

/// Normalized runtime state for a single VM.
///
/// This is a *reduced* state derived from `d2b list` + `d2b status`,
/// never a raw passthrough of either. Inconsistent or unreadable inputs reduce
/// to [`RuntimeState::Unknown`] (never to a false-healthy `Running`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeState {
    /// Process alive and (where applicable) api-ready.
    Running,
    /// Process alive but readiness not yet confirmed.
    Starting,
    /// Stop in progress.
    Stopping,
    /// Declared but not running.
    Stopped,
    /// State could not be determined.
    #[default]
    Unknown,
}

/// A USBIP busid claim, mirrored from `d2b usb probe`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbClaim {
    /// VM the claim is declared for.
    pub vm: String,
    /// Environment the claim belongs to.
    pub env: String,
    /// Host USB busid in canonical `B-P[.P...]` form.
    pub bus_id: String,
    /// Whether the device is currently bound.
    pub bound: bool,
    /// The VM currently holding the device, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_vm: Option<String>,
}

/// Per-VM feature toggles surfaced for display and control gating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VmFeatures {
    pub graphics: bool,
    pub tpm: bool,
    pub usbip: bool,
    /// True when the VM declares `audio.enable`. Controls are enabled only
    /// after d2b also reports live audio status for the VM.
    pub audio: bool,
}

/// Runtime operations the connected d2b daemon says this VM supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VmCapabilities {
    pub start: bool,
    pub stop: bool,
    pub restart: bool,
    pub switch: bool,
    pub build: bool,
    pub boot: bool,
    pub usb_hotplug: bool,
    pub store_verify: bool,
    pub terminal: bool,
}

impl Default for VmCapabilities {
    fn default() -> Self {
        Self {
            start: true,
            stop: true,
            restart: true,
            switch: true,
            build: true,
            boot: true,
            usb_hotplug: true,
            store_verify: true,
            terminal: true,
        }
    }
}

/// The audio channel represented by a d2b audio operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioChannel {
    Speaker,
    Microphone,
}

/// Provider backing a VM's audio controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioProviderKind {
    LocalHypervisor,
    QemuMedia,
    AcaSandbox,
    Unknown,
}

/// Enforcement posture reported by d2b for a VM's audio controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioEnforcementPosture {
    HostAndGuest,
    HostOnly,
    GuestOnly,
    Unsupported,
    Unknown,
}

/// Per-channel audio state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioChannelState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u16>,
    pub muted: bool,
}

/// Per-VM audio state and provider posture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VmAudioState {
    pub speaker: AudioChannelState,
    pub microphone: AudioChannelState,
    pub provider_kind: AudioProviderKind,
    pub enforcement: AudioEnforcementPosture,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
}

/// A custom per-VM quick-launch icon surfaced by the popup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLaunchIcon {
    pub id: String,
    pub icon: String,
    pub tooltip: String,
}

/// Runtime provider backing a public workload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WorkloadProviderKind {
    LocalVm,
    QemuMedia,
    ProviderManaged,
    UnsafeLocal,
    #[default]
    Unknown,
}

impl WorkloadProviderKind {
    pub fn is_unsafe_local(self) -> bool {
        self == Self::UnsafeLocal
    }
}

/// Isolation boundary advertised for a workload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum IsolationPosture {
    VirtualMachine,
    ProviderManaged,
    UnsafeLocal,
    #[default]
    Unknown,
}

/// Environment ownership advertised for workload execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum EnvironmentPosture {
    RuntimeManaged,
    SystemdUserManagerAmbient,
    #[default]
    Unknown,
}

/// Display-environment boundary advertised for workload execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DisplayEnvironmentPosture {
    RuntimeManaged,
    WaylandProxyOnly,
    NotApplicable,
    #[default]
    Unknown,
}

/// Identity used to execute a workload item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionIdentityPosture {
    WorkloadUser,
    ProviderManaged,
    AuthenticatedRequesterUid,
    #[default]
    Unknown,
}

/// Lifetime boundary advertised for launched workload sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SessionPersistencePosture {
    RuntimeManaged,
    UserManagerLifetime,
    #[default]
    Unknown,
}

/// Structured execution posture from d2b's public workload status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkloadExecutionPosture {
    #[serde(default)]
    pub isolation: IsolationPosture,
    #[serde(default)]
    pub environment: EnvironmentPosture,
    #[serde(default)]
    pub display_environment: DisplayEnvironmentPosture,
    #[serde(default)]
    pub execution_identity: ExecutionIdentityPosture,
    #[serde(default)]
    pub session_persistence: SessionPersistencePosture,
}

/// Provider readiness for configured launch and shell operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WorkloadAvailability {
    Ready,
    HelperUnavailable,
    HelperStale,
    UserManagerUnavailable,
    GraphicalSessionInactive,
    WaylandUnavailable,
    ProxyUnavailable,
    Degraded,
    #[default]
    Unknown,
}

impl WorkloadAvailability {
    pub fn remediation(self) -> Option<&'static str> {
        match self {
            Self::Ready => None,
            Self::HelperUnavailable => Some(
                "Unsafe-local helper unavailable; enable and start the d2b unsafe-local user service.",
            ),
            Self::HelperStale => {
                Some("Unsafe-local helper is stale; restart the d2b unsafe-local user service.")
            }
            Self::UserManagerUnavailable => Some(
                "User manager unavailable; sign in through a graphical PAM session and start systemd --user.",
            ),
            Self::GraphicalSessionInactive => {
                Some("Graphical session inactive; sign in to the target Wayland session.")
            }
            Self::WaylandUnavailable => {
                Some("Wayland unavailable; restore the graphical session before launching.")
            }
            Self::ProxyUnavailable => {
                Some("Wayland proxy unavailable; restart the d2b desktop user services.")
            }
            Self::Degraded => Some("Workload provider is degraded; inspect d2b workload status."),
            Self::Unknown => Some("Workload availability was not reported by d2b."),
        }
    }
}

/// Runtime state from the public workload inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WorkloadRuntimeState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Failed,
    #[default]
    Unknown,
}

/// Generic configured launcher-item kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum LauncherItemKind {
    Exec,
    Shell,
    #[default]
    Unknown,
}

/// Public presentation icon owned by a configured launcher item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LauncherIcon {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl LauncherIcon {
    pub fn preferred(&self) -> &str {
        self.name
            .as_deref()
            .or(self.id.as_deref())
            .unwrap_or("apps")
    }
}

/// One configured public launcher item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LauncherItemSummary {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: LauncherIcon,
    #[serde(default, rename = "type")]
    pub kind: LauncherItemKind,
    #[serde(default)]
    pub graphical: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
}

/// A public workload summary within a [`RealmGroup`].
///
/// The historical launcher fields remain for serialized compatibility. New
/// renderers consume the provider, posture, availability, and generic
/// `launcher_items` fields populated from d2b's public workload inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RealmLauncherEntry {
    /// Historical default action ID.
    pub action_id: String,
    /// Workload name within the realm.
    pub workload_name: String,
    /// Display label.
    pub label: String,
    /// XDG icon name.
    pub icon: String,
    /// Canonical target address for this workload.
    pub canonical_target: String,
    /// Legacy d2b VM name backing this workload, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_vm_name: Option<String>,
    /// True when another workload in the same realm shares this icon.
    ///
    /// The UI layer must show a chooser rather than launching directly.
    #[serde(default)]
    pub has_icon_collision: bool,
    /// action_ids of all workloads in the same realm sharing the same icon.
    /// Empty when `has_icon_collision` is false.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub icon_siblings: Vec<String>,
    /// Realm presentation name and stable ID from public workload identity.
    #[serde(default)]
    pub realm_name: String,
    #[serde(default)]
    pub realm_id: String,
    /// Runtime provider and structured execution posture.
    #[serde(default)]
    pub provider_kind: WorkloadProviderKind,
    #[serde(default)]
    pub execution_posture: WorkloadExecutionPosture,
    /// Current provider readiness and workload runtime state.
    #[serde(default)]
    pub availability: WorkloadAvailability,
    #[serde(default)]
    pub workload_state: WorkloadRuntimeState,
    /// Known and forward-compatible unknown capability tokens.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
    /// Configured exec and shell items owned by this workload.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub launcher_items: Vec<LauncherItemSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_item_id: Option<String>,
}

impl RealmLauncherEntry {
    pub fn is_unsafe_local(&self) -> bool {
        self.provider_kind.is_unsafe_local()
            || self.execution_posture.isolation == IsolationPosture::UnsafeLocal
    }

    pub fn warning(&self) -> Option<&'static str> {
        self.is_unsafe_local().then_some(
            "No isolation; processes run as your host user for the user-manager lifetime.",
        )
    }
}

/// A realm group in the quick-launch surface.
///
/// The outer/group border color is `realm_color`; workload inner borders use
/// default or theme styling unless explicitly overridden.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RealmGroup {
    /// Realm name as declared in `d2b.realms.<name>`.
    pub realm_name: String,
    /// Stable realm ID.
    pub realm_id: String,
    /// Resolved accent color for the outer group border (`#rrggbb`).
    ///
    /// Sourced from `ui-colors.json` envs accent when the realm name matches
    /// a declared env; otherwise derived from the realm name using the d2b
    /// color palette hash.
    pub realm_color: String,
    /// Workload entries within this realm, in declaration order.
    pub workloads: Vec<RealmLauncherEntry>,
}

impl RealmGroup {
    pub fn all_unsafe_local(&self) -> bool {
        !self.workloads.is_empty()
            && self
                .workloads
                .iter()
                .all(RealmLauncherEntry::is_unsafe_local)
    }

    pub fn has_mixed_isolation(&self) -> bool {
        self.workloads
            .iter()
            .any(RealmLauncherEntry::is_unsafe_local)
            && self
                .workloads
                .iter()
                .any(|workload| !workload.is_unsafe_local())
    }
}

/// A normalized VM as presented to the UI surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Vm {
    /// VM name as declared in `d2b.vms.<name>`.
    pub name: String,
    /// Canonical realm target asserted by d2b, when known.
    ///
    /// Local VMs default to `<vm>.local.d2b` during the realm-native
    /// transition. UI surfaces should prefer this for trusted VM identity and
    /// keep guest app ids/titles as presentation metadata only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_target: Option<String>,
    /// Environment name, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    /// Reduced runtime state.
    pub state: RuntimeState,
    /// True for framework-declared net VMs (`sys-*-net`); hidden by default.
    #[serde(default)]
    pub is_net_vm: bool,
    /// True when user config hides this VM from compact surfaces.
    #[serde(default)]
    pub hidden: bool,
    /// True when the running closure differs from the declared closure.
    #[serde(default)]
    pub pending_restart: bool,
    /// Declared feature toggles.
    #[serde(default)]
    pub features: VmFeatures,
    /// Runtime operation support reported by d2b.
    #[serde(default)]
    pub capabilities: VmCapabilities,
    /// Static IP, when declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub static_ip: Option<String>,
    /// Free-form readiness/role hints for the detail view.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readiness: Vec<String>,
    /// USB claims associated with this VM.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub usb: Vec<UsbClaim>,
    /// Audio status and controls, when d2b reports them for this VM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<VmAudioState>,
    /// Configured custom quick-launch icons for this VM.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quick_launch: Vec<QuickLaunchIcon>,
}

/// Connectivity / authorization posture for the whole control surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Connectivity {
    /// Public socket reachable and a role was resolved.
    Connected,
    /// Public socket reachable but no role (controls are read-only/denied).
    AuthDenied,
    /// `d2bd` is unreachable.
    #[default]
    DaemonDown,
}

/// The aggregate, reduced control-surface state. This is what every UI surface
/// renders and what `d2b-wlcontrol status-json` emits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WlState {
    /// Connectivity / auth posture.
    pub connectivity: Connectivity,
    /// Effective operator role.
    pub role: AuthRole,
    /// All known VMs (including net VMs and hidden ones); renderers use
    /// `is_net_vm` / `hidden` to choose compact vs. detail surfaces.
    pub vms: Vec<Vm>,
    /// Realm groups populated from d2b's public workload inventory.
    ///
    /// Each group carries a realm-scoped accent color for the outer border;
    /// inner workload borders use theme/default styling. Empty when the
    /// public workload operation is unavailable or empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub realm_groups: Vec<RealmGroup>,
    /// True when this state was served from cache after a failed refresh.
    #[serde(default)]
    pub stale: bool,
    /// Optional human-facing note (e.g. last error remediation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl WlState {
    /// Count of running VMs, excluding net VMs.
    pub fn running_count(&self) -> usize {
        self.vms
            .iter()
            .filter(|v| !v.is_net_vm && !v.hidden && v.state == RuntimeState::Running)
            .count()
    }

    /// Count of visible (non-net, non-hidden) VMs.
    pub fn visible_count(&self) -> usize {
        self.vms
            .iter()
            .filter(|v| !v.is_net_vm && !v.hidden)
            .count()
    }

    /// True when any visible VM needs operator attention (pending restart or
    /// an unknown/inconsistent state while the daemon is reachable).
    pub fn needs_attention(&self) -> bool {
        if self.connectivity != Connectivity::Connected {
            return false;
        }
        self.vms
            .iter()
            .filter(|v| !v.is_net_vm && !v.hidden)
            .any(|v| {
                v.pending_restart
                    || v.state == RuntimeState::Unknown
                    || v.audio
                        .as_ref()
                        .is_some_and(|audio| audio.error_kind.is_some() || !audio.microphone.muted)
            })
            || self.realm_groups.iter().any(|group| {
                group.workloads.iter().any(|workload| {
                    !matches!(
                        workload.availability,
                        WorkloadAvailability::Ready | WorkloadAvailability::Unknown
                    )
                })
            })
    }
}

/// The set of operations the control surface can request. Each maps to a
/// d2b public-socket request or an argv-only host process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
pub enum ActionKind {
    /// Refresh the reduced state.
    Refresh,
    /// Start a VM (`--apply`).
    Start { vm: String },
    /// Stop a VM (`--apply`).
    Stop { vm: String },
    /// Force stop a VM, bypassing graceful guest shutdown.
    ForceStop { vm: String },
    /// Restart a VM (`--apply`).
    Restart { vm: String },
    /// Activate the VM's current closure (`switch --apply`).
    Switch { vm: String },
    /// Build/evaluate the per-VM toplevel without activation.
    Build { vm: String },
    /// Stage the current per-VM closure for next boot (`boot --apply`).
    Boot { vm: String },
    /// Bind a USB busid to a VM (`usb attach --apply`).
    UsbAttach { vm: String, bus_id: String },
    /// Unbind a USB busid from a VM (`usb detach --apply`).
    UsbDetach { vm: String, bus_id: String },
    /// Verify the per-VM store live pool.
    StoreVerify { vm: String },
    /// Launch a guest terminal with detached guest-control exec.
    LaunchTerminal { vm: String },
    /// Run a configured custom guest quick-launch command.
    QuickLaunch { vm: String, id: String },
    /// Toggle microphone forwarding for a VM.
    AudioMic { vm: String, on: bool },
    /// Toggle speaker forwarding for a VM.
    AudioSpeaker { vm: String, on: bool },
    /// Set speaker playback volume for a VM.
    AudioSpeakerVolume { vm: String, level_percent: u8 },
    /// Set microphone input gain for a VM.
    AudioMicGain { vm: String, level_percent: u8 },
    /// Disable all audio forwarding for a VM.
    AudioOff { vm: String },
    /// Open / focus the Quickshell control center.
    OpenControlCenter,
    /// Open the configured observability portal in a browser.
    OpenObservability,
    /// Cycle the Waybar compact/detail display mode.
    CycleDisplay,
    /// Historical realm launcher action retained for serialized compatibility.
    ///
    /// New callers use [`ActionKind::WorkloadLaunch`]. The planner rejects this
    /// variant rather than consulting private launcher artifacts.
    RealmWorkloadLaunch {
        realm_id: String,
        action_id: String,
        /// Workload name, carried for display/audit purposes.
        workload_name: String,
    },
    /// Dispatch one configured public workload launcher item.
    WorkloadLaunch {
        target: String,
        item_id: String,
        item_kind: LauncherItemKind,
    },
}

/// Why an action is or is not currently available.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "reason")]
pub enum Unavailable {
    /// `d2bd` is unreachable.
    DaemonDown,
    /// Caller role is insufficient for this action.
    InsufficientRole { required: AuthRole },
    /// The target VM is not in a state that allows the action.
    VmState { detail: String },
    /// USB device is owned by another VM.
    UsbOwnedElsewhere { owner: String },
    /// Backed by a d2b surface that is not yet implemented.
    NotYetImplemented,
    /// Generic block with a human-facing detail.
    Blocked { detail: String },
}

/// An action paired with whether it can currently be invoked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionAvailability {
    pub action: ActionKind,
    /// `None` means available; `Some(_)` carries the block reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<Unavailable>,
}

impl ActionAvailability {
    pub fn available(action: ActionKind) -> Self {
        Self {
            action,
            unavailable: None,
        }
    }

    pub fn blocked(action: ActionKind, reason: Unavailable) -> Self {
        Self {
            action,
            unavailable: Some(reason),
        }
    }

    pub fn is_available(&self) -> bool {
        self.unavailable.is_none()
    }
}

/// A fully-resolved, ready-to-dispatch action.
///
/// The planner emits exactly one of these. A [`PlannedAction::Process`] is an
/// **argv vector**, never a shell string — there is no shell interpolation
/// anywhere in the control surface. `wait` tells the CLI whether to wait for
/// short-lived commands (build, detached exec creation) or just launch and
/// return (browser open).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "dispatch")]
pub enum PlannedAction {
    /// A d2b public-socket intent the protocol client should execute.
    Socket { intent: SocketIntent },
    /// A host process, expressed as an argv vector.
    Process {
        argv: Vec<String>,
        #[serde(default)]
        wait: bool,
    },
}

/// A typed d2b public-socket intent. The protocol client maps each variant
/// onto the corresponding `PublicRequest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "intent")]
pub enum SocketIntent {
    List,
    Status {
        vm: String,
    },
    AuthStatus,
    UsbProbe,
    VmStart {
        vm: String,
    },
    VmStop {
        vm: String,
        #[serde(default, skip_serializing_if = "is_false")]
        force: bool,
    },
    VmRestart {
        vm: String,
    },
    Switch {
        vm: String,
    },
    Boot {
        vm: String,
    },
    UsbAttach {
        vm: String,
        bus_id: String,
    },
    UsbDetach {
        vm: String,
        bus_id: String,
    },
    StoreVerify {
        vm: String,
    },
    AudioMute {
        vm: String,
        channel: AudioChannel,
        mute: bool,
    },
    AudioSetVolume {
        vm: String,
        channel: AudioChannel,
        level_percent: u8,
    },
    AudioOff {
        vm: String,
    },
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_and_visible_counts_exclude_net_vms() {
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Admin,
            vms: vec![
                Vm {
                    name: "corp-vm".into(),
                    canonical_target: None,
                    env: Some("work".into()),
                    state: RuntimeState::Running,
                    is_net_vm: false,
                    hidden: false,
                    pending_restart: false,
                    features: VmFeatures::default(),
                    capabilities: VmCapabilities::default(),
                    static_ip: None,
                    readiness: vec![],
                    usb: vec![],
                    audio: None,
                    quick_launch: vec![],
                },
                Vm {
                    name: "sys-work-net".into(),
                    canonical_target: None,
                    env: Some("work".into()),
                    state: RuntimeState::Running,
                    is_net_vm: true,
                    hidden: false,
                    pending_restart: false,
                    features: VmFeatures::default(),
                    capabilities: VmCapabilities::default(),
                    static_ip: None,
                    readiness: vec![],
                    usb: vec![],
                    audio: None,
                    quick_launch: vec![],
                },
            ],
            stale: false,
            note: None,
            ..Default::default()
        };
        assert_eq!(state.running_count(), 1);
        assert_eq!(state.visible_count(), 1);
        assert!(!state.needs_attention());
    }

    #[test]
    fn attention_triggers_on_pending_restart() {
        let mut state = WlState {
            connectivity: Connectivity::Connected,
            ..Default::default()
        };
        state.vms.push(Vm {
            name: "corp-vm".into(),
            canonical_target: None,
            env: None,
            state: RuntimeState::Running,
            is_net_vm: false,
            hidden: false,
            pending_restart: true,
            features: VmFeatures::default(),
            capabilities: VmCapabilities::default(),
            static_ip: None,
            readiness: vec![],
            usb: vec![],
            audio: None,
            quick_launch: vec![],
        });
        assert!(state.needs_attention());
    }

    #[test]
    fn attention_triggers_on_active_microphone() {
        let mut state = WlState {
            connectivity: Connectivity::Connected,
            ..Default::default()
        };
        state.vms.push(Vm {
            name: "corp-vm".into(),
            audio: Some(VmAudioState {
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
            }),
            ..Default::default()
        });
        assert!(state.needs_attention());
    }

    #[test]
    fn counts_and_attention_exclude_hidden_vms() {
        let state = WlState {
            connectivity: Connectivity::Connected,
            role: AuthRole::Admin,
            vms: vec![Vm {
                name: "noisy-vm".into(),
                canonical_target: None,
                env: None,
                state: RuntimeState::Unknown,
                is_net_vm: false,
                hidden: true,
                pending_restart: true,
                features: VmFeatures::default(),
                capabilities: VmCapabilities::default(),
                static_ip: None,
                readiness: vec![],
                usb: vec![],
                audio: None,
                quick_launch: vec![],
            }],
            stale: false,
            note: None,
            ..Default::default()
        };
        assert_eq!(state.running_count(), 0);
        assert_eq!(state.visible_count(), 0);
        assert!(!state.needs_attention());
    }

    #[test]
    fn wlstate_round_trips_through_json() {
        let mut state = WlState::default();
        state.vms.push(Vm {
            name: "corp-vm".into(),
            hidden: true,
            ..Default::default()
        });
        let json = serde_json::to_string(&state).expect("serialize");
        let back: WlState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(state, back);
    }

    #[test]
    fn vm_hidden_defaults_when_absent_from_json() {
        let vm: Vm = serde_json::from_str(
            r#"{
                "name": "corp-vm",
                "state": "running"
            }"#,
        )
        .expect("deserialize vm");
        assert!(!vm.hidden);
    }

    #[test]
    fn audio_action_variants_round_trip_through_json() {
        let actions = [
            ActionKind::Build {
                vm: "corp-vm".into(),
            },
            ActionKind::Boot {
                vm: "corp-vm".into(),
            },
            ActionKind::ForceStop {
                vm: "corp-vm".into(),
            },
            ActionKind::AudioMic {
                vm: "corp-vm".into(),
                on: true,
            },
            ActionKind::AudioSpeaker {
                vm: "corp-vm".into(),
                on: false,
            },
            ActionKind::AudioSpeakerVolume {
                vm: "corp-vm".into(),
                level_percent: 70,
            },
            ActionKind::AudioMicGain {
                vm: "corp-vm".into(),
                level_percent: 40,
            },
            ActionKind::AudioOff {
                vm: "corp-vm".into(),
            },
        ];

        for action in actions {
            let json = serde_json::to_string(&action).expect("serialize action");
            let back: ActionKind = serde_json::from_str(&json).expect("deserialize action");
            assert_eq!(action, back);
        }

        let json = serde_json::to_string(&ActionKind::OpenObservability).expect("serialize action");
        let back: ActionKind = serde_json::from_str(&json).expect("deserialize action");
        assert_eq!(back, ActionKind::OpenObservability);
    }

    #[test]
    fn vm_stop_socket_intent_defaults_force_false_for_compatibility() {
        let intent: SocketIntent =
            serde_json::from_str(r#"{"intent":"vm-stop","vm":"corp-vm"}"#).expect("deserialize");
        assert_eq!(
            intent,
            SocketIntent::VmStop {
                vm: "corp-vm".into(),
                force: false
            }
        );

        let normal_json = serde_json::to_string(&intent).expect("serialize");
        assert!(!normal_json.contains("force"));

        let force_json = serde_json::to_string(&SocketIntent::VmStop {
            vm: "corp-vm".into(),
            force: true,
        })
        .expect("serialize");
        assert!(force_json.contains(r#""force":true"#));
    }

    #[test]
    fn historical_realm_launcher_fields_default_new_workload_contract() {
        let entry: RealmLauncherEntry = serde_json::from_str(
            r#"{
                "actionId":"browser",
                "workloadName":"browser",
                "label":"Browser",
                "icon":"web",
                "canonicalTarget":"browser.work.d2b",
                "hasIconCollision":false
            }"#,
        )
        .expect("historical launcher entry");
        assert_eq!(entry.provider_kind, WorkloadProviderKind::Unknown);
        assert_eq!(entry.availability, WorkloadAvailability::Unknown);
        assert!(entry.launcher_items.is_empty());
    }

    #[test]
    fn generic_workload_action_round_trips_with_typed_item_kind() {
        let action = ActionKind::WorkloadLaunch {
            target: "tools.host.d2b".to_owned(),
            item_id: "terminal".to_owned(),
            item_kind: LauncherItemKind::Shell,
        };
        let encoded = serde_json::to_string(&action).expect("serialize workload action");
        let decoded: ActionKind = serde_json::from_str(&encoded).expect("deserialize action");
        assert_eq!(decoded, action);
    }
}
