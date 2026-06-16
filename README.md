# nixling-wlcontrol

A clean, Waybar-styled indicator and control center for
[nixling](https://github.com/vicondoa/nixling) microVMs, built for a
niri / Wayland desktop.

`nixling-wlcontrol` shows which nixling VMs are running and surfaces the
controls a nixling operator can already drive — start / stop / restart,
launch a terminal into a VM, attach / detach USB, and verify a VM's
store — without ever touching anything privileged. It talks only to the
nixlingd **public** socket and, where it is the better boundary, the
official `nixling` CLI.

> Status: early development. The Waybar indicator, reduced status model,
> and action planning are in place; the live public-socket client and
> GTK control center are landing wave by wave (see
> [AGENTS.md](./AGENTS.md)).

## What it does

- **Glanceable status in Waybar** — a compact `◆ 2/4` style indicator
  with state-driven CSS classes (`all-running`, `partial-running`,
  `attention`, `daemon-down`, `auth-denied`) and a per-VM tooltip.
- **A GTK4/libadwaita control center** — per-VM cards with lifecycle
  controls, terminal launch, USB attach/detach, and store verify, all
  gated on your effective nixling authorization.
- **Safe by construction** — public socket only; no broker socket, no
  `sudo`, no direct state-file mutation, argv-only command execution.

Audio (mic / speaker) controls are designed but **disabled** until
nixling exposes a daemon-native audio control plane — today those
nixling verbs return `not-yet-implemented`.

## Install

`nixling-wlcontrol` is a Nix flake.

```bash
# Run it directly
nix run github:vicondoa/nixling-wlcontrol -- status-json

# Or add it as an input and install the package
#   inputs.nixling-wlcontrol.url = "github:vicondoa/nixling-wlcontrol";
# then add `nixling-wlcontrol.packages.${system}.default` to your packages.
```

For development:

```bash
nix develop          # rust toolchain + GTK system deps
cargo test --workspace
```

## Waybar setup

Print a starter module config and CSS:

```bash
nixling-wlcontrol print-waybar-config   # add to your Waybar "modules" + a "modules-*" array
nixling-wlcontrol print-css             # append to your style.css
```

The module is a continuous custom module — it loops and emits one JSON
line per refresh, so do **not** give it an `interval`. Left-click opens
the control center, right-click cycles the display mode, middle-click
refreshes. See [docs/waybar.md](./docs/waybar.md).

## niri setup

An optional floating-window rule for the control center lives in
[`data/niri-window-rule.kdl`](./data/niri-window-rule.kdl). See
[docs/niri.md](./docs/niri.md).

## Configuration

Configuration is TOML at
`${XDG_CONFIG_HOME:-~/.config}/nixling-wlcontrol/config.toml`. All
defaults are sane; the most common override is your terminal command.
See [docs/configuration.md](./docs/configuration.md).

## Documentation

- [docs/configuration.md](./docs/configuration.md) — config options.
- [docs/controls.md](./docs/controls.md) — the action matrix + auth gating.
- [docs/waybar.md](./docs/waybar.md) — Waybar module + styling.
- [docs/niri.md](./docs/niri.md) — niri / Wayland integration.
- [docs/security.md](./docs/security.md) — trust boundary + command safety.
- [AGENTS.md](./AGENTS.md) — contributor / agent operating manual.

## License

[Apache-2.0](./LICENSE).
