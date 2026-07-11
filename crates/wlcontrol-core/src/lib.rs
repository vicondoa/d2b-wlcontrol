//! `wlcontrol-core` — the frozen domain contract, configuration, reducer, and
//! action planner shared by every d2b-wlcontrol surface.
//!
//! See [`model`] for the cross-crate contract that downstream fleet agents
//! build against.

pub mod config;
pub mod error;
pub mod model;
pub mod plan;
pub mod realm_launcher;
pub mod reduce;
pub mod sources;

pub use config::{
    is_public_socket_path, load_ui_colors_from_path, Config, LauncherOverride, UiColorArtifact,
    UiColorBorder, UiColorEnv, UiColorHost, UiColorStates, UiColorVm, WaybarConfig,
    DEFAULT_COLOR_ARTIFACT_PATH,
};
pub use error::{WlError, WlResult};
pub use model::{
    ActionAvailability, ActionKind, AuthRole, Connectivity, EnvironmentPosture,
    ExecutionIdentityPosture, IsolationPosture, LauncherIcon, LauncherItemKind,
    LauncherItemSummary, PlannedAction, RealmGroup, RealmLauncherEntry, RuntimeState,
    SessionPersistencePosture, SocketIntent, Unavailable, UsbClaim, Vm, VmFeatures, WlState,
    WorkloadAvailability, WorkloadExecutionPosture, WorkloadProviderKind, WorkloadRuntimeState,
};
pub use realm_launcher::build_realm_groups;
