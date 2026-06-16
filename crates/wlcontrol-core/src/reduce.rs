//! State reduction and precedence.
//!
//! Owning wave: **Wave 1 — Core model agent**. Wave 0 ships a correct baseline
//! so downstream surfaces have real data to render; the Wave 1 agent hardens
//! the precedence rules, net-VM detection, and inconsistency → attention
//! mapping per the plan's "State model" section.
//!
//! Precedence contract:
//! 1. `inventory` (`nixling list`) defines the declared VM set, env, features,
//!    static IP, and default order.
//! 2. `statuses` (`nixling status <vm>`) override runtime state, readiness, and
//!    pending-restart.
//! 3. `usb` (`nixling usb probe`) attaches USB claims.
//! 4. `auth` (`nixling auth status`) sets the effective role.
//! 5. Missing/inconsistent inputs reduce to `Unknown`, never false-healthy.

use crate::model::{AuthRole, Connectivity, RuntimeState, Vm, WlState};
use crate::sources::{InventoryVm, ReduceInput, VmStatus};

/// Reduce a bundle of source fragments into the aggregate [`WlState`].
pub fn reduce(input: ReduceInput) -> WlState {
    if input.connectivity != Connectivity::Connected {
        return WlState {
            connectivity: input.connectivity,
            role: AuthRole::None,
            vms: Vec::new(),
            stale: false,
            note: None,
        };
    }

    let role = input.auth.map(|a| a.role).unwrap_or(AuthRole::None);
    let connectivity = if role == AuthRole::None {
        Connectivity::AuthDenied
    } else {
        Connectivity::Connected
    };

    let inventory = input.inventory.unwrap_or_default();
    let usb_claims = input.usb.map(|u| u.claims).unwrap_or_default();

    let vms = inventory
        .vms
        .into_iter()
        .map(|inv| build_vm(inv, &input.statuses, &usb_claims))
        .collect();

    WlState {
        connectivity,
        role,
        vms,
        stale: false,
        note: None,
    }
}

fn build_vm(inv: InventoryVm, statuses: &[VmStatus], usb_claims: &[crate::model::UsbClaim]) -> Vm {
    let status = statuses.iter().find(|s| s.name == inv.name);

    // Per-VM status is authoritative; fall back to the coarse list status.
    let state = match status {
        Some(s) => s.state,
        None => coarse_state(inv.coarse_status.as_deref()),
    };

    let usb = usb_claims
        .iter()
        .filter(|c| c.vm == inv.name)
        .cloned()
        .collect();

    Vm {
        name: inv.name,
        env: inv.env,
        state,
        is_net_vm: inv.is_net_vm,
        pending_restart: status.map(|s| s.pending_restart).unwrap_or(false),
        features: inv.features,
        static_ip: inv.static_ip,
        readiness: status.map(|s| s.readiness.clone()).unwrap_or_default(),
        usb,
    }
}

fn coarse_state(s: Option<&str>) -> RuntimeState {
    match s {
        Some(v) if v.starts_with("running") => RuntimeState::Running,
        Some(v) if v.starts_with("stopped") => RuntimeState::Stopped,
        Some(_) => RuntimeState::Unknown,
        None => RuntimeState::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::{Auth, Inventory, UsbProbe};

    #[test]
    fn daemon_down_yields_empty_state() {
        let input = ReduceInput {
            connectivity: Connectivity::DaemonDown,
            ..Default::default()
        };
        let state = reduce(input);
        assert_eq!(state.connectivity, Connectivity::DaemonDown);
        assert!(state.vms.is_empty());
        assert_eq!(state.role, AuthRole::None);
    }

    #[test]
    fn per_vm_status_overrides_coarse() {
        let input = ReduceInput {
            connectivity: Connectivity::Connected,
            auth: Some(Auth {
                role: AuthRole::Admin,
            }),
            inventory: Some(Inventory {
                vms: vec![InventoryVm {
                    name: "corp-vm".into(),
                    env: Some("work".into()),
                    is_net_vm: false,
                    features: Default::default(),
                    static_ip: None,
                    coarse_status: Some("stopped".into()),
                }],
            }),
            statuses: vec![VmStatus {
                name: "corp-vm".into(),
                state: RuntimeState::Running,
                pending_restart: true,
                readiness: vec!["api-ready".into()],
            }],
            usb: Some(UsbProbe::default()),
        };
        let state = reduce(input);
        assert_eq!(state.vms.len(), 1);
        assert_eq!(state.vms[0].state, RuntimeState::Running);
        assert!(state.vms[0].pending_restart);
        assert_eq!(state.role, AuthRole::Admin);
    }

    #[test]
    fn no_role_maps_to_auth_denied() {
        let input = ReduceInput {
            connectivity: Connectivity::Connected,
            auth: Some(Auth {
                role: AuthRole::None,
            }),
            inventory: Some(Inventory::default()),
            ..Default::default()
        };
        let state = reduce(input);
        assert_eq!(state.connectivity, Connectivity::AuthDenied);
    }
}
