# Controls and action matrix

Every mutating control is gated on two things:

1. **Connectivity** — `nixlingd` must be reachable on the public socket.
2. **Authorization** — your effective role from `nixling auth status`
   (`none`, `launcher`, or `admin`). The control surface never guesses
   authorization from filesystem permissions.

When an action is unavailable, the UI says *why* (daemon down,
insufficient role, VM not in a runnable state, USB owned elsewhere, or
unsupported by the current nixling control plane) rather than failing
silently.

## Action matrix

| Action | Default | Min role | Backing surface | Notes |
| --- | --- | --- | --- | --- |
| Show declared VMs | on | none | `nixling list` | VM set, env, features, order. |
| Show per-VM runtime | on | none | `nixling status <vm>` | Runtime/readiness/pending-restart truth. |
| USB probe | on | none | `nixling usb probe` | Read-only claim/ownership view. |
| Start / Stop / Restart | on | launcher | `vm start|stop|restart --apply` | Explicit apply; gated on VM state. |
| Switch (activate closure) | advanced | launcher | `switch --apply` | Confirm if VM is running. |
| USB attach | on | launcher | `usb attach --apply` | Only when unbound/ownerless and VM ready. |
| USB detach | on | launcher | `usb detach --apply` | Only for the owning VM. |
| Store verify | advanced | launcher | `store verify` | Detail-panel action. |
| Launch terminal | on | admin | terminal + `nixling vm exec -it <vm> -- <shell>` | Admin-only guest exec; argv-only. |
| Audio mic / speaker / off | **disabled** | — | `nixling audio …` | Disabled until nixling's audio plane is live. |
| Host install/destroy/migrate/keys | hidden | — | nixling CLI | Out of scope for a control surface. |

## Role gating

- `none` → read-only. The bar shows `auth-denied`; controls explain the
  missing authorization.
- `launcher` → lifecycle + USB + store verify.
- `admin` → everything launcher can do, plus terminal/guest exec.

## Audio is intentionally disabled

nixling's `audio mic|speaker|off|status` verbs currently return a typed
`not-yet-implemented` envelope, and nixling explicitly has no
daemon-native audio control plane yet. `nixling-wlcontrol` renders these
controls **disabled with a clear reason** and never edits
`audio-state.json` directly. When nixling ships a working audio surface,
these controls light up with no UI redesign.

The control center renders this matrix with auth-aware gating: blocked
actions are disabled with a tooltip explaining why, and destructive
actions (stop/restart/switch on a running VM) prompt for confirmation.
