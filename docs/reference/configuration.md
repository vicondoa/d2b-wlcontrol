# Configuration

`d2b-wlcontrol` reads
`${XDG_CONFIG_HOME:-~/.config}/d2b-wlcontrol/config.toml`. A missing file uses
defaults; a malformed file is an error.

```toml
public_socket = "/run/d2b/public.sock"
refresh_interval_ms = 2500
command_timeout_ms = 10000
hide_net_vms = true
show_pending_restart = true
favorites = ["builder"]
hidden_vms = []

[terminal]
# Existing per-VM guest-terminal control.
guest_argv = ["/run/current-system/sw/bin/foot"]

# Persistent-shell launcher used by public `shell` items.
wlterm_argv = ["d2b-wlterm", "open"]

[waybar]
icon = "◆"
label = ""

[theme]
background = "#0f1117"
surface = "#16181d"
surface_alt = "#2a2d35"
foreground = "#cdd6f4"
foreground_strong = "#ffffff"
foreground_disabled = "#bac2de"
muted = "#9399b2"
border = "#2a2d35"
inverse_foreground = "#000000"
success_surface = "#1a2e1a"
warning_surface = "#2e2a1a"
error_surface = "#2e1a1a"
input_background = "#0d0d0d"
slider_track = "#252832"

[[launcher_overrides]]
target = "tools.host.d2b"
item_id = "firefox"
name = "Web"
icon = "language"
```

## Public workload launchers

Workload identity, provider/posture, readiness, and configured launch items are
read from d2bd's public workload operation. There is no launcher artifact path
option. Each public item supplies its own `name`, icon, and typed `exec` or
`shell` kind.

`[[launcher_overrides]]` changes presentation only. It is keyed by canonical
target plus item ID and may replace `name` and/or `icon`; dispatch still uses the
public target and item ID. Do not use overrides to invent undeclared commands.

Exec items run through exact argv:

```text
d2b launch <canonical-target> --item <item-id>
```

Shell items append the canonical target and item ID to
`terminal.wlterm_argv`:

```text
d2b-wlterm open <canonical-target> <item-id>
```

This is the persistent-shell/wlterm path, not `d2b vm exec`.

## Legacy per-VM controls

`terminal.guest_argv` and `[[quick_launch]]` remain available for ordinary local
VM cards:

```toml
[[quick_launch]]
id = "run-tool"
vm = "builder"
icon = "construction"
tooltip = "Run tool"
guest_argv = ["/run/current-system/sw/bin/tool"]
```

These argv vectors are never shell strings. Unsafe-local workload rows never
receive these VM/guest-exec controls.

## Colors

The `[theme]` table is a Stylix-agnostic neutral shell palette. Values must be
normalized lowercase `#rrggbb`. d2b state, realm, and VM accents still come
from the configured public UI color artifact (`/etc/d2b/ui-colors.json` by
default). Realm accents prefer first-class realm metadata, then matching
environment metadata, then the deterministic d2b palette.

## Host Home Manager module

```nix
{
  imports = [ inputs.d2b-wlcontrol.homeManagerModules.default ];

  programs.d2b-wlcontrol = {
    enable = true;
    publicSocketPath = "/run/d2b/public.sock";
    colorArtifactPath = "/etc/d2b/ui-colors.json";

    launcherOverrides = [
      {
        target = "tools.host.d2b";
        itemId = "firefox";
        name = "Web";
        icon = "language";
      }
    ];

    waybar = {
      enable = true;
      barName = "mainBar";
      modulesList = "modules-right";
      icon = "◆";
      label = "d2b";
      clickAction = "d2b-wlcontrol open";
      module = {
        "on-click-right" = "d2b-wlcontrol action cycle-display";
      };
    };
  };
}
```

The module writes the TOML, installs the package and CSS, owns the configurable
public d2b color-artifact path, injects the module at the selected position, and
leaves arbitrary raw TOML/Waybar overrides available through `settings` and
`waybar.module`. The neutral theme stays independent and Stylix-agnostic. The
module never imports d2b's guest Home Manager module.

## Input alignment

The flake pins the `d2b-client-toolkit` source distribution exactly at
`800c2878533f600d8f085b3d2aafcddb970232b2`, backed by canonical d2b revision
`4018d9c9652bd826c2e6a9abccdcdcafb832d944`, distribution fingerprint
`c2c99bdd77ba66948fce81161dcc3efde608eefefb96f28fa934c9f58d96d838`, and
inventory digest
`2aaef697cc53abc8757a3593352cd5bd1d3f0d3f2031c6a2967f92afa5e74d97`.
Consumers composing desktop companions should keep one client-toolkit revision:

```nix
inputs.d2b-wlcontrol.inputs.d2b-client-toolkit.follows = "d2b-client-toolkit";
inputs.d2b-wlterm.inputs.d2b-client-toolkit.follows = "d2b-client-toolkit";
```
