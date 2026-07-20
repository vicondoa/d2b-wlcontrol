# Controls and action matrix

Availability combines d2bd public-socket connectivity, the effective
`none`/`launcher`/`admin` role, public workload capabilities/readiness, and VM
runtime capabilities. Disabled controls explain the failed prerequisite.

| Surface | Minimum role | Dispatch |
| --- | --- | --- |
| Local VM inventory/status | authenticated launcher/admin | canonical `DaemonClient::Inspect` |
| Configured `exec` item | launcher | `d2b launch <target> --item <id>` argv |
| Configured `shell` item | launcher | configured `d2b-wlterm open <target> <id>` argv |
| VM start/stop/restart | admin | canonical typed daemon lifecycle methods |
| VM build | launcher | `d2b build <vm>` argv |
| VM boot/switch/store verify | admin | d2bd public operations |
| VM USB attach/detach | admin | d2bd public USB operations |
| VM audio controls | admin | d2bd public audio operation |
| Existing VM terminal/quick launch | admin | detached guest-control exec |
| Observability URL | none | configured browser argv |

All process dispatch is argv-only.

The authenticated local session currently proves only that d2bd admitted a
configured launcher or admin. It does not project which role was selected, so
wlcontrol reports the conservative launcher posture and keeps admin controls
disabled. The lifecycle mapping is ready for the canonical role projection; it
does not guess admin authority from socket ownership or group membership.

USB, store, audio, desktop observer/action, and User/Shell/Notify/Wayland
service kinds do not yet have runtime-owned client routes. Those actions fail
before connecting and never fall back to legacy JSON, root-owned state, helper
sockets, or direct compositor control.

## Generic items

Every configured item is rendered from its own public `name` and icon. Firefox,
OpenObserve, terminals, and other applications are ordinary items; the UI does
not carry application-specific fields or variants. An item's typed kind selects
only the dispatch boundary:

- `exec` uses d2b configured launch;
- `shell` uses wlterm/persistent-shell with the canonical workload target.

VM-backed items appear as icon actions on the existing compact VM row alongside
its lifecycle controls; wlcontrol does not add a second workload card for the
same VM. Non-VM workloads, including unsafe-local host tools, use the same
compact row hierarchy without VM-only controls. Tooltips retain each item-owned
display name.

## Unsafe-local control policy

Unsafe-local is intentionally not a VM. Its rows show:

- configured launcher items;
- provider and availability status;
- the no-isolation/user-manager-lifetime warning; and
- actionable helper/session/Wayland remediation.

They do **not** show VM lifecycle, build/boot/switch, store verification, USB,
audio, the VM terminal button, per-VM quick launch, or arbitrary guest exec.
The planner also rejects VM-shaped actions if an unsafe workload is matched by
canonical target or a legacy VM hint.

When every workload in a realm card is unsafe-local, one card-level warning is
shown. A mixed card places the warning only on its unsafe rows.

## Readiness failures

| Availability | Operator guidance |
| --- | --- |
| `helper-unavailable` | Enable and start the d2b unsafe-local user service. |
| `helper-stale` | Restart the user helper so it matches d2bd. |
| `user-manager-unavailable` | Enter a graphical PAM session and restore `systemd --user`. |
| `graphical-session-inactive` | Sign in to the target graphical session. |
| `wayland-unavailable` | Restore the Wayland session. |
| `proxy-unavailable` | Restart d2b desktop user services; there is no direct-compositor fallback. |
| `degraded` | Inspect `d2b` workload status. |

## Existing VM controls

Local VM cards retain lifecycle, USB, store, audio, build/switch, and
guest-terminal controls. Destructive VM operations keep confirmation behavior.
Audio remains daemon-native and never reads or mutates audio state files.
