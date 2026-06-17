# Security model

`nixling-wlcontrol` is a presentation + control surface. It holds no
privilege of its own and is designed so a bug cannot escalate into a
host compromise.

## Trust boundary

The tool talks **only** to operator-facing surfaces:

- the nixlingd **public** socket `/run/nixling/public.sock`
  (non-abstract `SOCK_SEQPACKET`, 4-byte little-endian length-prefixed
  JSON frames, `SO_PEERCRED` authorization); and
- the official `nixling` CLI, used only where it is the better boundary
  (detached guest terminal exec and non-shell build); and
- the configured browser opener for the observability URL.

Authorization is whatever nixlingd grants the calling user via
`SO_PEERCRED` + group membership. `nixling-wlcontrol` adds **no**
privilege and enforces no policy of its own beyond hiding controls the
daemon would reject anyway.

## Hard rules

- **No broker socket.** Never connects to `/run/nixling/priv.sock`.
- **No privilege escalation.** Never uses `sudo` or setuid paths.
- **No direct state mutation.** Never reads or writes nixling's
  root-owned state files (e.g. `audio-state.json`); all state changes go
  through the public socket or the `nixling` CLI.
- **argv-only execution.** Every spawned process is an argv vector. No
  shell, no string interpolation, so VM names / bus ids / shell paths
  can never become shell metacharacters.
- **Auth from the daemon, not the filesystem.** Control availability is
  derived from `nixling auth status`, never from inspecting file
  permissions.
- **No XWayland assumptions.**
- **No observability credential handling.** The Signoz button opens a URL only;
  auto-login/token/cookie handling is out of scope.

## Failure posture

- `nixlingd` unreachable → `daemon-down` state; mutating controls
  disabled, not errored mid-flight.
- Reachable but unauthorized → `auth-denied` state; read-only.
- A failed refresh reuses the last state marked `stale` rather than
  flapping to a false-healthy or empty view.
- nixling typed errors and remediation text are surfaced to the
  operator; raw command output and any secrets are never logged.

## Reporting

Security concerns about `nixling-wlcontrol` should be reported privately
to the repository owner. Issues in nixling itself belong in the
[nixling](https://github.com/vicondoa/nixling) project.
