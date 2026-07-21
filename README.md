# d2b-wlcontrol

A Waybar indicator and Quickshell control center for
[d2b](https://github.com/vicondoa/d2b) on niri and other Wayland
desktops.

`d2b-wlcontrol` owns the presentation model, reducer, Waybar output, and
Quickshell control center for d2b workloads. Its 2.0 client boundary consumes
the exact canonical d2b client source distributed by `d2b-client-toolkit`;
repository-local transport and wire definitions are not supported.

## Highlights

- **Presentation-owned workload cards.** Canonical targets, provider posture,
  availability, and configured items remain normalized into repository-local
  view models without redefining their wire representation.
- **Generic launcher items.** Each item owns its label and icon. Browser,
  observability, terminal, and application entries are ordinary configured
  items rather than hardcoded UI variants.
- **Honest unsafe-local UX.** All-unsafe cards carry one card warning; mixed
  cards warn only on unsafe rows. The UI says that these processes have no
  isolation and live for the user-manager lifetime. VM lifecycle, build/switch,
  storage, USB, audio, and arbitrary guest-exec controls are not shown for
  unsafe-local rows.
- **Actionable readiness.** Missing/stale helpers, unavailable user managers,
  inactive graphical sessions, Wayland failures, and proxy failures explain the
  remediation instead of silently failing.
- **Safe dispatch.** Configured exec uses exact argv
  `d2b launch <target> --item <id>`. Shell items use the configured
  `d2b-wlterm open <target> <item-id>` persistent-shell boundary. No shell
  interpolation is used.
- **Waybar contract.** The module self-loops and emits one newline-terminated
  JSON object per refresh with bounded classes. Unsafe posture may add only the
  stable `unsafe-local` / `mixed-isolation` classes.
- **d2b colors.** Realm accents and VM/state colors still use d2b's public UI
  color metadata from a configurable artifact path; neutral popup colors remain
  independently configurable and Stylix-agnostic.

The canonical client adapter now establishes an authenticated ComponentSession
to the fixed local daemon endpoint and consumes its typed inspection read
model. Frozen start/stop/restart requests use the typed daemon client. The
session does not expose the caller's admin mapping, so wlcontrol conservatively
enables only launcher-level controls; admin controls stay disabled. Desktop
observer/action, User/Shell/Notify/Wayland, device, audio, and other routes that
still depend on integrated runtime routing fail closed with no legacy fallback.

## Trust boundary

The control center talks to `/run/d2b/public.sock`, invokes the official `d2b`
CLI for exact configured launch/build boundaries, invokes `d2b-wlterm` for
persistent shells, and uses the configured browser opener. The socket carries
only the canonical authenticated ComponentSession service. Wlcontrol never
contacts the broker socket, invokes `sudo`, reads private helper state, or
reads root-owned d2b launcher/state files.

## Install

```nix
{
  inputs.d2b-client-toolkit = {
    url = "github:vicondoa/d2b-toolkit/926de54e7320599c373524a10b65aaf13b6ff422";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  inputs.d2b-wlcontrol.url = "github:vicondoa/d2b-wlcontrol";
  inputs.d2b-wlcontrol.inputs.nixpkgs.follows = "nixpkgs";
  inputs.d2b-wlcontrol.inputs.d2b-client-toolkit.follows = "d2b-client-toolkit";
}
```

This source pins client-toolkit commit
`926de54e7320599c373524a10b65aaf13b6ff422`, canonical d2b revision
`9dc902243cdd7aba7ef269988b96f0aae6e037da`, distribution fingerprint
`5a20cef3a64281df819eeb76bdfe385999755479b467b559653011582fb9c043`, and
inventory digest
`35c33c2e23e1b9f03b5abc3bbca2d3320e38c42dfc7aceb7e3476d28210cde8c`.

Install `inputs.d2b-wlcontrol.packages.${system}.default`, or use the host Home
Manager module:

```nix
{
  imports = [ inputs.d2b-wlcontrol.homeManagerModules.default ];

  programs.d2b-wlcontrol = {
    enable = true;
    colorArtifactPath = "/etc/d2b/ui-colors.json";
    waybar = {
      enable = true;
      modulesList = "modules-right";
      icon = "◆";
      label = "";
    };
  };
}
```

The module installs wlcontrol, writes its TOML, owns the configurable public d2b
color-artifact path, installs the starter Waybar CSS, injects the custom module
without an `interval`, and preserves module placement, click-action,
icon/label, CSS, and launcher-item overrides. It remains a host module and does
not import d2b's guest-only Home Manager component or Stylix.

## Waybar and niri

Without Home Manager:

```bash
d2b-wlcontrol print-waybar-config
d2b-wlcontrol print-css
```

Install the package on `PATH`, add `custom/d2b-wlcontrol` to a Waybar module
list, and do not configure `interval`. Left-click toggles the native
Quickshell layer-shell popup; no niri window rule or XWayland is required.
The popup starts unpinned, closes on focus loss after activation, and can be
pinned or dragged within the compositor-provided usable output area.

## Development

```bash
export PATH="$(echo ~/.rustup/toolchains/1.94.1-*/bin):/home/paydro/.nix-profile/bin:$PATH"
export CARGO_BUILD_RUSTC_WRAPPER=''
export CARGO_TARGET_DIR="$PWD/.cargo-target"

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
nix flake check --no-build --all-systems
```

With a live niri/Wayland session, generate the deterministic production-tree
UI review sample as a 420×640 physical-pixel PNG with:

```bash
d2b-wlcontrol render-sample --output "$PWD/wlcontrol-panel.png"
```

## Documentation

- [Configuration](docs/reference/configuration.md)
- [Presentation model](docs/reference/presentation-model.md)
- [Controls](docs/reference/controls.md)
- [Waybar](docs/how-to/configure-waybar.md)
- [niri / Wayland](docs/how-to/configure-niri.md)
- [Security](docs/explanation/security.md)
- [Troubleshooting](docs/how-to/troubleshooting.md)
- [Contributor manual](AGENTS.md)

## License

[Apache-2.0](LICENSE)
