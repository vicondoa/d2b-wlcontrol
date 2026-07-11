# Security model

`d2b-wlcontrol` is an unprivileged presentation and control client.

## Allowed boundaries

- d2bd's public `SOCK_SEQPACKET` socket, including public VM/audio/USB and
  workload list/status operations;
- exact official `d2b` argv for configured launch and build;
- configured `d2b-wlterm` argv for persistent workload shells; and
- the configured browser opener.

Public-socket authorization remains `SO_PEERCRED` plus d2b's reported role.
Wlcontrol never infers privilege from filesystem permissions.

## Forbidden boundaries

- `/run/d2b/priv.sock` or any helper/broker socket;
- `sudo`, setuid helpers, `su`, or guessed user-session credentials;
- root-owned d2b state, private launcher artifacts, helper registrations, argv,
  environment, command output, or session records; and
- shell command strings/interpolation.

The only d2b file consumed for presentation is the public UI color metadata.
It is never treated as authorization or policy.

## Configured launch

Public launcher records contain identity, presentation, posture, capabilities,
and item IDs—not executable paths or argv. Exec dispatch is the fixed vector:

```text
["d2b", "launch", TARGET, "--item", ITEM_ID]
```

Shell dispatch appends `TARGET` and `ITEM_ID` to a configured wlterm argv
prefix. It never falls back to `d2b vm exec` for unsafe-local.

## Unsafe-local means unsafe

An unsafe-local provider executes as the authenticated host user and has no VM
or provider isolation boundary. Its session follows the user's systemd manager.
The realm rail and Wayland proxy identify the workload but do not sandbox it.

Wlcontrol prevents misleading controls by omitting VM lifecycle, storage, USB,
audio, build/switch, and arbitrary guest-exec actions from unsafe-local rows.
This is UX defense-in-depth; d2bd remains the authorization and dispatch
authority.

## Failure posture

- daemon unavailable: mutating controls disabled;
- auth denied: read-only display;
- helper stale/unavailable or user manager absent: launch disabled with
  remediation;
- Wayland/proxy unavailable: graphical launch disabled, never bypassed to the
  host compositor;
- failed launch process: non-zero status reaches the UI result boundary; and
- failed refresh: cached state is marked stale rather than false-healthy.

Waybar classes are fixed low-cardinality values. Raw targets, names, argv,
paths, output, environment, and shell names are not metric/CSS labels.
