//! Canonical authenticated d2b client adapter boundary.
//!
//! Local daemon inspection and lifecycle calls use the exact service clients
//! distributed by `d2b-client-toolkit`. Desktop observer/action routing remains
//! unavailable until the integrated runtime supplies its authenticated route.

use std::{
    collections::BTreeSet,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    os::fd::{AsFd, AsRawFd, OwnedFd},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use d2b_client_toolkit::contracts::{
    v2_identity::{RealmId, RealmPath},
    v2_services::{common, daemon, MAX_PAGE_SIZE},
};
use d2b_client_toolkit::{
    daemon_call_options, local_daemon_endpoint_identity, CancellationToken, Client, ClientError,
    DaemonClient, DaemonLifecycleRequest, DaemonMethod, HandshakeCredentials, HostSocketConnector,
    RouteRecord, RouteTable, ServiceKind, ServiceOwner, TargetInput, TransportKind,
    TransportSelection,
};
use nix::{
    errno::Errno,
    poll::{poll, PollFd, PollFlags, PollTimeout},
    sys::socket::{
        connect, getsockopt, socket, sockopt, AddressFamily, SockFlag, SockType, UnixAddr,
    },
    unistd::{Gid, Uid, User},
};
use sha2::{Digest, Sha256};
use tokio::runtime::{Builder, Runtime};
use wlcontrol_core::{
    error::{WlError, WlResult},
    model::{AuthRole, Connectivity, RuntimeState, SocketIntent, VmCapabilities, VmFeatures},
    sources::{Auth, Inventory, InventoryVm, ReduceInput, VmStatus},
    Config,
};

pub use d2b_client_toolkit::{
    D2B_SOURCE_FINGERPRINT as CLIENT_SOURCE_FINGERPRINT,
    D2B_SOURCE_REVISION as CLIENT_SOURCE_REVISION,
};

const MAX_PAGES: usize = 1024;
const ROUTING_UNAVAILABLE: &str =
    "the frozen canonical service has no authenticated route for this operation";

/// Bounded retry budget for a nonblocking `connect(2)` that races an
/// `AF_UNIX` `SOCK_SEQPACKET` listener whose backlog is momentarily full
/// (Linux reports this as `EAGAIN`, not `ECONNREFUSED`). Each attempt is a
/// fresh `connect(2)` syscall, not a poll cycle.
const CONNECT_RETRY_ATTEMPTS: u32 = 20;
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(10);
/// Bounded budget for polling a connect-in-progress (`EINPROGRESS`) fd for
/// writability before treating the connect as failed.
const CONNECT_POLL_ATTEMPTS: u32 = 50;
const CONNECT_POLL_TIMEOUT_MS: u16 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchOutcome {
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct D2bClient {
    socket_path: String,
    timeout: Duration,
}

impl D2bClient {
    pub fn new(config: &Config) -> Self {
        Self {
            socket_path: config.public_socket.clone(),
            timeout: Duration::from_millis(config.command_timeout_ms),
        }
    }

    pub fn socket_path(&self) -> &str {
        &self.socket_path
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn refresh(&self) -> ReduceInput {
        self.refresh_result().unwrap_or_else(|_| ReduceInput {
            connectivity: Connectivity::DaemonDown,
            ..Default::default()
        })
    }

    fn refresh_result(&self) -> WlResult<ReduceInput> {
        let runtime = client_runtime()?;
        let projections = runtime
            .block_on(async {
                tokio::time::timeout(self.timeout, async {
                    let daemon = connect_daemon(&self.socket_path).await?;
                    inspect_all(&daemon, None).await
                })
                .await
            })
            .map_err(|_| WlError::Timeout(format!("{:?}", self.timeout)))?
            .map_err(map_client_error)?;
        Ok(reduce_input(projections))
    }

    pub fn dispatch(&self, intent: &SocketIntent) -> WlResult<DispatchOutcome> {
        let plan = DispatchPlan::from_intent(intent)?;
        let runtime = client_runtime()?;
        runtime
            .block_on(async {
                tokio::time::timeout(self.timeout, async {
                    let daemon = connect_daemon(&self.socket_path).await?;
                    dispatch_plan(&daemon, plan).await
                })
                .await
            })
            .map_err(|_| WlError::Timeout(format!("{:?}", self.timeout)))?
            .map_err(map_client_error)
    }
}

enum DispatchPlan<'a> {
    Inspect(Option<&'a str>),
    Lifecycle {
        method: DaemonMethod,
        desired_state: common::DesiredState,
        resource_id: &'a str,
        operation: &'static str,
    },
}

impl<'a> DispatchPlan<'a> {
    fn from_intent(intent: &'a SocketIntent) -> WlResult<Self> {
        let plan = match intent {
            SocketIntent::List => Self::Inspect(None),
            SocketIntent::Status { vm } => Self::Inspect(Some(vm)),
            SocketIntent::VmStart { vm } => Self::Lifecycle {
                method: DaemonMethod::Start,
                desired_state: common::DesiredState::DESIRED_STATE_RUNNING,
                resource_id: vm,
                operation: "start",
            },
            SocketIntent::VmStop { vm, force: false } => Self::Lifecycle {
                method: DaemonMethod::Stop,
                desired_state: common::DesiredState::DESIRED_STATE_STOPPED,
                resource_id: vm,
                operation: "stop",
            },
            SocketIntent::VmRestart { vm } => Self::Lifecycle {
                method: DaemonMethod::Restart,
                desired_state: common::DesiredState::DESIRED_STATE_RUNNING,
                resource_id: vm,
                operation: "restart",
            },
            SocketIntent::AuthStatus
            | SocketIntent::UsbProbe
            | SocketIntent::VmStop { force: true, .. }
            | SocketIntent::Switch { .. }
            | SocketIntent::Boot { .. }
            | SocketIntent::UsbAttach { .. }
            | SocketIntent::UsbDetach { .. }
            | SocketIntent::StoreVerify { .. }
            | SocketIntent::AudioMute { .. }
            | SocketIntent::AudioSetVolume { .. }
            | SocketIntent::AudioOff { .. } => {
                return Err(WlError::D2b(ROUTING_UNAVAILABLE.to_owned()));
            }
        };
        Ok(plan)
    }
}

async fn dispatch_plan(
    daemon: &DaemonClient,
    plan: DispatchPlan<'_>,
) -> Result<DispatchOutcome, ClientError> {
    match plan {
        DispatchPlan::Inspect(resource_id) => {
            inspect_all(daemon, resource_id).await?;
            Ok(DispatchOutcome {
                summary: "d2b state refreshed".to_owned(),
            })
        }
        DispatchPlan::Lifecycle {
            method,
            desired_state,
            resource_id,
            operation,
        } => {
            let operation_id = operation_id(operation);
            let request_digest =
                lifecycle_digest(operation, resource_id, desired_state, &operation_id);
            daemon
                .lifecycle(
                    DaemonLifecycleRequest {
                        method,
                        resource_id,
                        desired_state,
                        operation_id: &operation_id,
                        request_digest,
                    },
                    daemon_call_options(true)?,
                    &CancellationToken::default(),
                )
                .await?;
            Ok(DispatchOutcome {
                summary: format!("{operation} accepted for {resource_id}"),
            })
        }
    }
}

fn client_runtime() -> WlResult<Runtime> {
    Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .map_err(|_| WlError::D2b("failed to start canonical client runtime".to_owned()))
}

async fn connect_daemon(socket_path: &str) -> Result<DaemonClient, ClientError> {
    let path = PathBuf::from(socket_path);
    let (fd, client_uid, client_gid, daemon_uid) =
        tokio::task::spawn_blocking(move || connect_seqpacket(&path))
            .await
            .map_err(|_| ClientError::ConnectFailed)??;
    let identity = local_daemon_endpoint_identity(client_uid, client_gid)?;
    let connector =
        HostSocketConnector::from_seqpacket_fd(fd, daemon_uid, identity, HandshakeCredentials::Nn)?;
    let realm_path = RealmPath::parse("local-root").map_err(|_| ClientError::InvalidTarget)?;
    let realm = RealmId::derive(&realm_path);
    let connected = Client::new(
        RouteTable::new(vec![RouteRecord {
            owner: ServiceOwner::LocalRoot(realm.clone()),
            transport: TransportKind::LocalUnix,
        }]),
        connector,
    )
    .connect(
        TargetInput::LocalRoot(realm),
        ServiceKind::Daemon,
        TransportSelection::exact(TransportKind::LocalUnix),
    )
    .await?;
    DaemonClient::new(connected)
}

fn connect_seqpacket(path: &Path) -> Result<(OwnedFd, u32, u32, u32), ClientError> {
    let fd = socket(
        AddressFamily::Unix,
        SockType::SeqPacket,
        SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK,
        None,
    )
    .map_err(|_| ClientError::ConnectFailed)?;
    let address = UnixAddr::new(path).map_err(|_| ClientError::ConnectFailed)?;
    connect_nonblocking(&fd, &address)?;
    let daemon_uid = User::from_name("d2bd")
        .map_err(|_| ClientError::ConnectFailed)?
        .ok_or(ClientError::ConnectFailed)?
        .uid
        .as_raw();
    Ok((
        fd,
        Uid::effective().as_raw(),
        Gid::effective().as_raw(),
        daemon_uid,
    ))
}

/// Drives a nonblocking `connect(2)` on `fd` to completion.
///
/// The fd is created with `SOCK_NONBLOCK` (it is later handed to the async
/// runtime, so it must stay nonblocking), which means `connect(2)` on an
/// `AF_UNIX SOCK_SEQPACKET` socket can legitimately return:
///
/// - `EINPROGRESS`: the connect is in flight; poll for writability, then
///   resolve the real outcome via `SO_ERROR`.
/// - `EAGAIN` (Linux-specific for `AF_UNIX`): the listener's backlog is
///   momentarily full; retry the `connect(2)` call itself after a short,
///   bounded delay.
/// - `EINTR`: retry immediately.
///
/// Any other error, or exhausting the bounded retry/poll budget, is
/// reported as `ClientError::ConnectFailed` rather than treated as fatal
/// on the first syscall return.
fn connect_nonblocking(fd: &OwnedFd, address: &UnixAddr) -> Result<(), ClientError> {
    for attempt in 0..CONNECT_RETRY_ATTEMPTS {
        match connect(fd.as_raw_fd(), address) {
            Ok(()) => return Ok(()),
            Err(Errno::EINPROGRESS) => return wait_for_connect(fd),
            Err(Errno::EINTR) => continue,
            Err(Errno::EAGAIN) => {
                if attempt + 1 == CONNECT_RETRY_ATTEMPTS {
                    return Err(ClientError::ConnectFailed);
                }
                std::thread::sleep(CONNECT_RETRY_DELAY);
            }
            Err(_) => return Err(ClientError::ConnectFailed),
        }
    }
    Err(ClientError::ConnectFailed)
}

/// Polls a connect-in-progress fd for writability and resolves the
/// outcome via `SO_ERROR`, within a bounded number of poll attempts.
fn wait_for_connect(fd: &OwnedFd) -> Result<(), ClientError> {
    let mut fds = [PollFd::new(fd.as_fd(), PollFlags::POLLOUT)];
    for _ in 0..CONNECT_POLL_ATTEMPTS {
        match poll(&mut fds, PollTimeout::from(CONNECT_POLL_TIMEOUT_MS)) {
            Ok(0) => continue,
            Ok(_) => {
                let err =
                    getsockopt(fd, sockopt::SocketError).map_err(|_| ClientError::ConnectFailed)?;
                return if err == 0 {
                    Ok(())
                } else {
                    Err(ClientError::ConnectFailed)
                };
            }
            Err(Errno::EINTR) => continue,
            Err(_) => return Err(ClientError::ConnectFailed),
        }
    }
    Err(ClientError::ConnectFailed)
}

async fn inspect_all(
    daemon: &DaemonClient,
    resource_id: Option<&str>,
) -> Result<Vec<daemon::WorkloadProjection>, ClientError> {
    let cancellation = CancellationToken::default();
    let mut cursor = None;
    let mut seen = BTreeSet::new();
    let mut read_model = None;
    let mut projections = Vec::new();
    for _ in 0..MAX_PAGES {
        let response = daemon
            .inspect(
                resource_id,
                MAX_PAGE_SIZE,
                cursor.as_deref(),
                daemon_call_options(false)?,
                &cancellation,
            )
            .await?;
        if read_model
            .as_ref()
            .is_some_and(|current| current != &response.read_model)
        {
            return Err(ClientError::ContractViolation);
        }
        read_model.get_or_insert(response.read_model);
        let page = response
            .page
            .into_option()
            .ok_or(ClientError::ContractViolation)?;
        projections.extend(response.workloads);
        if !page.truncated {
            return Ok(projections);
        }
        if page.next_page_cursor.is_empty() || !seen.insert(page.next_page_cursor.clone()) {
            return Err(ClientError::ContractViolation);
        }
        cursor = Some(page.next_page_cursor);
    }
    Err(ClientError::ContractViolation)
}

fn reduce_input(projections: Vec<daemon::WorkloadProjection>) -> ReduceInput {
    let local_vms = projections
        .into_iter()
        .filter(is_vm_projection)
        .collect::<Vec<_>>();
    let inventory = Inventory {
        vms: local_vms.iter().map(inventory_vm).collect(),
    };
    let statuses = local_vms.iter().map(vm_status).collect();
    ReduceInput {
        connectivity: Connectivity::Connected,
        // A successfully negotiated local daemon session proves at least the
        // frozen launcher read authority. It does not prove admin authority.
        auth: Some(Auth {
            role: AuthRole::Launcher,
        }),
        inventory: Some(inventory),
        statuses,
        ..Default::default()
    }
}

fn is_vm_projection(workload: &daemon::WorkloadProjection) -> bool {
    matches!(
        workload
            .runtime
            .as_ref()
            .map(|runtime| runtime.kind.enum_value_or_default()),
        Some(
            daemon::RuntimeKind::RUNTIME_KIND_NIXOS | daemon::RuntimeKind::RUNTIME_KIND_QEMU_MEDIA
        )
    )
}

fn inventory_vm(workload: &daemon::WorkloadProjection) -> InventoryVm {
    let identity = workload.identity.as_ref();
    let name = projection_name(workload);
    let supported = supported_capabilities(workload);
    let runtime_kind = workload
        .runtime
        .as_ref()
        .map(|runtime| runtime.kind.enum_value_or_default());
    InventoryVm {
        name,
        canonical_target: nonempty(identity.map(|value| value.canonical_target.as_str())),
        env: nonempty(Some(workload.environment.as_str())),
        is_net_vm: workload.is_net_workload,
        features: VmFeatures {
            graphics: workload.graphics,
            tpm: workload.tpm,
            usbip: workload.usbip,
            audio: workload.services.iter().any(|service| {
                service.kind.enum_value_or_default() == daemon::ServiceKind::SERVICE_KIND_AUDIO
            }),
        },
        capabilities: capabilities(&supported, runtime_kind),
        static_ip: ip_address(&workload.static_ip),
        coarse_status: Some(runtime_state_name(runtime_state(workload)).to_owned()),
    }
}

fn vm_status(workload: &daemon::WorkloadProjection) -> VmStatus {
    let identity = workload.identity.as_ref();
    let supported = supported_capabilities(workload);
    let runtime_kind = workload
        .runtime
        .as_ref()
        .map(|runtime| runtime.kind.enum_value_or_default());
    let lifecycle = workload.lifecycle.as_ref();
    let mut readiness = lifecycle
        .into_iter()
        .flat_map(|value| &value.degraded_reasons)
        .map(|reason| {
            if reason.remediation.is_empty() {
                reason.reason.clone()
            } else {
                format!("{}: {}", reason.reason, reason.remediation)
            }
        })
        .collect::<Vec<_>>();
    readiness.extend(workload.readiness.iter().map(|item| {
        format!(
            "{}:{}",
            item.predicate_id,
            service_state_name(item.state.enum_value_or_default())
        )
    }));
    VmStatus {
        name: projection_name(workload),
        canonical_target: nonempty(identity.map(|value| value.canonical_target.as_str())),
        state: runtime_state(workload),
        pending_restart: lifecycle.is_some_and(|value| value.pending_restart),
        readiness,
        capabilities: capabilities(&supported, runtime_kind),
    }
}

fn projection_name(workload: &daemon::WorkloadProjection) -> String {
    if !workload.name.is_empty() {
        return workload.name.clone();
    }
    workload
        .identity
        .as_ref()
        .map(|identity| identity.workload_name.clone())
        .unwrap_or_default()
}

fn supported_capabilities(workload: &daemon::WorkloadProjection) -> Vec<daemon::RuntimeCapability> {
    workload
        .runtime
        .as_ref()
        .into_iter()
        .flat_map(|runtime| &runtime.supported_capabilities)
        .map(|capability| capability.enum_value_or_default())
        .collect()
}

fn capabilities(
    supported: &[daemon::RuntimeCapability],
    runtime_kind: Option<daemon::RuntimeKind>,
) -> VmCapabilities {
    let has = |capability| supported.contains(&capability);
    let lifecycle = has(daemon::RuntimeCapability::RUNTIME_CAPABILITY_LIFECYCLE);
    VmCapabilities {
        start: lifecycle,
        stop: lifecycle,
        restart: lifecycle,
        switch: false,
        build: runtime_kind == Some(daemon::RuntimeKind::RUNTIME_KIND_NIXOS),
        boot: false,
        usb_hotplug: false,
        store_verify: false,
        terminal: has(daemon::RuntimeCapability::RUNTIME_CAPABILITY_EXEC)
            || has(daemon::RuntimeCapability::RUNTIME_CAPABILITY_GUEST_CONTROL),
    }
}

fn runtime_state(workload: &daemon::WorkloadProjection) -> RuntimeState {
    match workload
        .lifecycle
        .as_ref()
        .map(|lifecycle| lifecycle.state.enum_value_or_default())
    {
        Some(daemon::WorkloadLifecycleState::WORKLOAD_LIFECYCLE_STATE_RUNNING) => {
            RuntimeState::Running
        }
        Some(
            daemon::WorkloadLifecycleState::WORKLOAD_LIFECYCLE_STATE_STARTING
            | daemon::WorkloadLifecycleState::WORKLOAD_LIFECYCLE_STATE_BOOTED
            | daemon::WorkloadLifecycleState::WORKLOAD_LIFECYCLE_STATE_RESTARTING,
        ) => RuntimeState::Starting,
        Some(daemon::WorkloadLifecycleState::WORKLOAD_LIFECYCLE_STATE_STOPPING) => {
            RuntimeState::Stopping
        }
        Some(daemon::WorkloadLifecycleState::WORKLOAD_LIFECYCLE_STATE_STOPPED) => {
            RuntimeState::Stopped
        }
        _ => RuntimeState::Unknown,
    }
}

fn runtime_state_name(state: RuntimeState) -> &'static str {
    match state {
        RuntimeState::Running => "running",
        RuntimeState::Starting => "starting",
        RuntimeState::Stopping => "stopping",
        RuntimeState::Stopped => "stopped",
        RuntimeState::Unknown => "unknown",
    }
}

fn service_state_name(state: daemon::ServiceState) -> &'static str {
    match state {
        daemon::ServiceState::SERVICE_STATE_ACTIVE => "active",
        daemon::ServiceState::SERVICE_STATE_INACTIVE => "inactive",
        daemon::ServiceState::SERVICE_STATE_STARTING => "starting",
        daemon::ServiceState::SERVICE_STATE_STOPPING => "stopping",
        daemon::ServiceState::SERVICE_STATE_FAILED => "failed",
        daemon::ServiceState::SERVICE_STATE_UNAVAILABLE => "unavailable",
        daemon::ServiceState::SERVICE_STATE_UNSUPPORTED => "unsupported",
        daemon::ServiceState::SERVICE_STATE_UNKNOWN
        | daemon::ServiceState::SERVICE_STATE_UNSPECIFIED => "unknown",
    }
}

fn nonempty(value: Option<&str>) -> Option<String> {
    value.filter(|value| !value.is_empty()).map(str::to_owned)
}

fn ip_address(bytes: &[u8]) -> Option<String> {
    match bytes {
        [a, b, c, d] => Some(IpAddr::V4(Ipv4Addr::new(*a, *b, *c, *d)).to_string()),
        bytes if bytes.len() == 16 => {
            let octets: [u8; 16] = bytes.try_into().ok()?;
            Some(IpAddr::V6(Ipv6Addr::from(octets)).to_string())
        }
        _ => None,
    }
}

fn operation_id(operation: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_nanos();
    format!("wlcontrol-{operation}-{}-{nanos}", std::process::id())
}

fn lifecycle_digest(
    operation: &str,
    resource_id: &str,
    desired_state: common::DesiredState,
    operation_id: &str,
) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(b"d2b-wlcontrol-lifecycle-v2\0");
    digest.update(operation.as_bytes());
    digest.update(resource_id.as_bytes());
    digest.update((desired_state as i32).to_be_bytes());
    digest.update(operation_id.as_bytes());
    digest.finalize().into()
}

fn map_client_error(error: ClientError) -> WlError {
    match error {
        ClientError::ConnectFailed | ClientError::SessionLost | ClientError::TransportFailed => {
            WlError::DaemonDown(error.to_string())
        }
        // The daemon is reachable; the authenticated session handshake
        // itself failed. Report the truthful remediation instead of a
        // blanket `DaemonDown`: an authentication rejection is `Denied`,
        // everything else (framing, schema, replay, resource-exhaustion,
        // and other handshake/record-protocol failures) is `Protocol`.
        ClientError::SessionEstablishment(code) => match code {
            d2b_client_toolkit::contracts::v2_component_session::SessionErrorCode::AuthenticationFailed => {
                WlError::Denied(error.to_string())
            }
            _ => WlError::Protocol(error.to_string()),
        },
        ClientError::Remote {
            kind:
                d2b_client_toolkit::RemoteErrorKind::Unauthorized
                | d2b_client_toolkit::RemoteErrorKind::Forbidden,
            ..
        } => WlError::Denied(error.to_string()),
        ClientError::DeadlineExpired => WlError::Timeout(error.to_string()),
        _ => WlError::D2b(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projection(
        name: &str,
        kind: daemon::RuntimeKind,
        state: daemon::WorkloadLifecycleState,
    ) -> daemon::WorkloadProjection {
        let mut identity = daemon::WorkloadIdentityProjection::new();
        identity.workload_name = name.to_owned();
        identity.canonical_target = format!("{name}.local.d2b");
        let mut runtime = daemon::RuntimeProjection::new();
        runtime.kind = kind.into();
        runtime.supported_capabilities = vec![
            daemon::RuntimeCapability::RUNTIME_CAPABILITY_LIFECYCLE.into(),
            daemon::RuntimeCapability::RUNTIME_CAPABILITY_EXEC.into(),
            daemon::RuntimeCapability::RUNTIME_CAPABILITY_USB_HOTPLUG.into(),
            daemon::RuntimeCapability::RUNTIME_CAPABILITY_STORE_SYNC.into(),
        ];
        let mut lifecycle = daemon::WorkloadLifecycleProjection::new();
        lifecycle.state = state.into();
        daemon::WorkloadProjection {
            identity: Some(identity).into(),
            name: name.to_owned(),
            environment: "work".to_owned(),
            graphics: true,
            static_ip: vec![10, 42, 0, 10],
            lifecycle: Some(lifecycle).into(),
            runtime: Some(runtime).into(),
            ..Default::default()
        }
    }

    #[test]
    fn binds_the_exact_frozen_service_source() {
        assert_eq!(
            CLIENT_SOURCE_REVISION,
            "9dc902243cdd7aba7ef269988b96f0aae6e037da"
        );
        assert_eq!(
            CLIENT_SOURCE_FINGERPRINT,
            "5a20cef3a64281df819eeb76bdfe385999755479b467b559653011582fb9c043"
        );
        assert!(matches!(ServiceKind::User, ServiceKind::User));
        assert!(matches!(ServiceKind::Shell, ServiceKind::Shell));
        assert!(matches!(ServiceKind::Notify, ServiceKind::Notify));
        assert!(matches!(ServiceKind::Wayland, ServiceKind::Wayland));
    }

    #[test]
    fn maps_canonical_vm_projection_without_wire_copies() {
        let input = reduce_input(vec![projection(
            "corp-vm",
            daemon::RuntimeKind::RUNTIME_KIND_NIXOS,
            daemon::WorkloadLifecycleState::WORKLOAD_LIFECYCLE_STATE_RUNNING,
        )]);
        assert_eq!(input.connectivity, Connectivity::Connected);
        assert_eq!(
            input.auth,
            Some(Auth {
                role: AuthRole::Launcher
            })
        );
        let vm = &input.inventory.expect("inventory").vms[0];
        assert_eq!(vm.name, "corp-vm");
        assert_eq!(vm.canonical_target.as_deref(), Some("corp-vm.local.d2b"));
        assert_eq!(vm.static_ip.as_deref(), Some("10.42.0.10"));
        assert!(vm.features.graphics);
        assert!(vm.capabilities.start);
        assert!(vm.capabilities.terminal);
        assert!(!vm.capabilities.usb_hotplug);
        assert!(!vm.capabilities.store_verify);
    }

    #[test]
    fn excludes_non_vm_workloads_from_vm_borders_and_controls() {
        let input = reduce_input(vec![projection(
            "host-tool",
            daemon::RuntimeKind::RUNTIME_KIND_UNSAFE_LOCAL,
            daemon::WorkloadLifecycleState::WORKLOAD_LIFECYCLE_STATE_RUNNING,
        )]);
        assert!(input.inventory.expect("inventory").vms.is_empty());
        assert!(input.statuses.is_empty());
        assert!(input.workloads.is_none());
    }

    #[test]
    fn maps_only_frozen_lifecycle_actions() {
        assert!(matches!(
            DispatchPlan::from_intent(&SocketIntent::VmStart {
                vm: "corp-vm".to_owned()
            })
            .expect("start plan"),
            DispatchPlan::Lifecycle {
                method: DaemonMethod::Start,
                ..
            }
        ));
        let client = D2bClient::new(&Config::default());
        let error = client
            .dispatch(&SocketIntent::UsbProbe)
            .expect_err("unrouted operation must fail before connecting");
        assert!(matches!(error, WlError::D2b(message) if message == ROUTING_UNAVAILABLE));
    }

    #[test]
    fn maps_session_authentication_failure_to_denied() {
        use d2b_client_toolkit::contracts::v2_component_session::SessionErrorCode;

        let error = map_client_error(ClientError::SessionEstablishment(
            SessionErrorCode::AuthenticationFailed,
        ));
        assert!(matches!(error, WlError::Denied(_)));
    }

    #[test]
    fn maps_other_session_establishment_codes_to_protocol() {
        use d2b_client_toolkit::contracts::v2_component_session::SessionErrorCode;

        for code in [
            SessionErrorCode::SchemaMismatch,
            SessionErrorCode::HandshakeTimeout,
            SessionErrorCode::RecordReplay,
        ] {
            let error = map_client_error(ClientError::SessionEstablishment(code));
            assert!(
                matches!(error, WlError::Protocol(_)),
                "expected Protocol for {code:?}, got {error:?}"
            );
        }
    }

    /// Binds and listens on a fresh `AF_UNIX SOCK_SEQPACKET` socket at a
    /// process-unique temporary path, mirroring the pattern already used by
    /// `wlcontrol-core`'s config tests for scratch filesystem state.
    fn listen_seqpacket(name: &str) -> (OwnedFd, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "d2b-wlcontrol-connect-test-{}-{}",
            std::process::id(),
            name
        ));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        let path = dir.join("d2bd.sock");
        let listener = socket(
            AddressFamily::Unix,
            SockType::SeqPacket,
            SockFlag::SOCK_CLOEXEC,
            None,
        )
        .expect("create listener socket");
        let address = UnixAddr::new(&path).expect("bind address");
        nix::sys::socket::bind(listener.as_raw_fd(), &address).expect("bind listener");
        nix::sys::socket::listen(&listener, nix::sys::socket::Backlog::new(1).unwrap())
            .expect("listen");
        (listener, path)
    }

    #[test]
    fn connect_nonblocking_succeeds_for_concurrent_clients() {
        let (listener, path) = listen_seqpacket("concurrent");
        let listener_fd = listener.as_raw_fd();
        let acceptor = std::thread::spawn(move || {
            for _ in 0..4 {
                let _ = nix::sys::socket::accept(listener_fd).expect("accept connection");
            }
            drop(listener);
        });

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let path = path.clone();
                std::thread::spawn(move || {
                    let fd = socket(
                        AddressFamily::Unix,
                        SockType::SeqPacket,
                        SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK,
                        None,
                    )
                    .expect("create client socket");
                    let address = UnixAddr::new(&path).expect("client address");
                    connect_nonblocking(&fd, &address)
                })
            })
            .collect();

        for handle in handles {
            handle
                .join()
                .expect("client thread")
                .expect("concurrent connect must succeed");
        }
        acceptor.join().expect("acceptor thread");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(path.parent().expect("parent dir"));
    }

    #[test]
    fn connect_nonblocking_retries_through_a_momentarily_full_backlog() {
        let (listener, path) = listen_seqpacket("backlog");
        let listener_fd = listener.as_raw_fd();
        // The listener is bound with a backlog of exactly one, and the
        // acceptor briefly delays before draining it: any client attempt
        // that races ahead of the first accept must observe the listener's
        // full backlog (Linux reports this as `EAGAIN` for `AF_UNIX`), not
        // a fatal error. `connect_nonblocking` must retry rather than
        // surface that transient condition immediately.
        let acceptor = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            for _ in 0..3 {
                let _ = nix::sys::socket::accept(listener_fd).expect("accept connection");
                std::thread::sleep(Duration::from_millis(5));
            }
            drop(listener);
        });

        let handles: Vec<_> = (0..3)
            .map(|_| {
                let path = path.clone();
                std::thread::spawn(move || {
                    let fd = socket(
                        AddressFamily::Unix,
                        SockType::SeqPacket,
                        SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK,
                        None,
                    )
                    .expect("create client socket");
                    let address = UnixAddr::new(&path).expect("client address");
                    connect_nonblocking(&fd, &address)
                })
            })
            .collect();

        for handle in handles {
            handle
                .join()
                .expect("client thread")
                .expect("connect must recover from a transiently full backlog");
        }
        acceptor.join().expect("acceptor thread");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(path.parent().expect("parent dir"));
    }
}
