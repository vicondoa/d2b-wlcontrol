# Configuration

`nixling-wlcontrol` reads TOML from
`${XDG_CONFIG_HOME:-~/.config}/nixling-wlcontrol/config.toml`. The file
is optional — every setting has a sane default. A present-but-malformed
file is a hard error (so you notice typos) rather than a silent
fallback to defaults.

## Example

```toml
# Path to the nixlingd public socket.
public_socket = "/run/nixling/public.sock"

# Waybar refresh cadence (ms) and per-operation timeout (ms).
refresh_interval_ms = 2500
command_timeout_ms = 4000

# Hide framework net VMs (sys-*-net) from the compact surfaces.
hide_net_vms = true

# Show the pending-restart marker.
show_pending_restart = true

[terminal]
# Terminal command as an ARGV VECTOR (never a shell string). The VM's
# `nixling vm exec -it <vm> -- <guest_shell>` invocation is appended.
argv = ["foot", "--"]
guest_shell = "bash"
```

## Options

| Key | Type | Default | Meaning |
| --- | --- | --- | --- |
| `public_socket` | string | `/run/nixling/public.sock` | nixlingd public socket path. |
| `refresh_interval_ms` | integer | `2500` | Waybar poll cadence. |
| `command_timeout_ms` | integer | `4000` | Per-operation deadline. |
| `hide_net_vms` | bool | `true` | Hide `sys-*-net` VMs from compact views. |
| `show_pending_restart` | bool | `true` | Surface the pending-restart marker. |
| `terminal.argv` | array of string | `["foot", "--"]` | Terminal argv prefix. |
| `terminal.guest_shell` | string | `bash` | Guest shell launched inside the VM. |

## Terminal command is argv, not a shell string

The terminal command is always an **argv vector**. `nixling-wlcontrol`
spawns `terminal.argv` directly (via `execvp`-style process spawning)
and appends `nixling vm exec -it <vm> -- <guest_shell>` as discrete
arguments. There is no shell, so VM names and shell paths can never be
interpreted as shell metacharacters.

Common terminals:

```toml
argv = ["foot", "--"]              # foot
argv = ["wezterm", "start", "--"]  # wezterm
argv = ["kitty", "--"]             # kitty
argv = ["alacritty", "-e"]         # alacritty
```

> Owning wave: Wave 1 expands validation (e.g. rejecting an empty
> `terminal.argv`) and favorites/ordering options described in the plan.
