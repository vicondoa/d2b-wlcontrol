# d2b-wlcontrol

A Waybar indicator and Quickshell control center for
[d2b](https://github.com/vicondoa/d2b) on niri and other Wayland
desktops.

`d2b-wlcontrol` consumes d2b's public workload inventory and groups local VMs,
provider-managed workloads, and explicitly unsafe host workloads by realm. It
keeps the existing VM lifecycle, USB, store, audio, and guest-terminal controls
while rendering configured workload launchers without private files or
VM-shaped assumptions.

## Highlights

- **Public workload cards.** Canonical target, provider, isolation/execution
  posture, availability, and every configured `exec` or `shell` item come from
  d2bd's public socket.
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
  color metadata; neutral popup colors remain independently configurable and
  Stylix-agnostic.

## Trust boundary

The control center talks to `/run/d2b/public.sock`, invokes the official `d2b`
CLI for exact configured launch/build boundaries, invokes `d2b-wlterm` for
persistent shells, and uses the configured browser opener. It never contacts
the broker socket, invokes `sudo`, reads private helper state, or reads
root-owned d2b launcher/state files.

## Install

```nix
{
  inputs.d2b-toolkit.url =
    "github:vicondoa/d2b-toolkit/v0.2.0";
  inputs.d2b-toolkit.inputs.nixpkgs.follows = "nixpkgs";

  inputs.d2b-wlcontrol.url = "github:vicondoa/d2b-wlcontrol";
  inputs.d2b-wlcontrol.inputs.nixpkgs.follows = "nixpkgs";
  inputs.d2b-wlcontrol.inputs.d2b-toolkit.follows = "d2b-toolkit";
}
```

Install `inputs.d2b-wlcontrol.packages.${system}.default`, or use the host Home
Manager module:

```nix
{
  imports = [ inputs.d2b-wlcontrol.homeManagerModules.default ];

  programs.d2b-wlcontrol = {
    enable = true;
    waybar = {
      enable = true;
      modulesList = "modules-right";
      icon = "◆";
      label = "";
    };
  };
}
```

The module installs wlcontrol, writes its TOML, installs the starter Waybar CSS,
injects the custom module without an `interval`, and preserves module placement,
click-action, icon/label, CSS, and launcher-item overrides. It is a host module;
it does not import d2b's guest-only Home Manager component.

## Waybar and niri

Without Home Manager:

```bash
d2b-wlcontrol print-waybar-config
d2b-wlcontrol print-css
```

Install the package on `PATH`, add `custom/d2b-wlcontrol` to a Waybar module
list, and do not configure `interval`. Left-click toggles the native
Quickshell layer-shell popup; no niri window rule or XWayland is required.

## Development

```bash
export PATH="$(echo ~/.rustup/toolchains/1.94.1-*/bin):/home/paydro/.nix-profile/bin:$PATH"
export CARGO_BUILD_RUSTC_WRAPPER=''
export CARGO_TARGET_DIR=/home/paydro/.cache/d2b-wlcontrol-target

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
nix flake check --no-build --all-systems
```

## Documentation

- [Configuration](docs/reference/configuration.md)
- [Controls](docs/reference/controls.md)
- [Waybar](docs/how-to/configure-waybar.md)
- [niri / Wayland](docs/how-to/configure-niri.md)
- [Security](docs/explanation/security.md)
- [Troubleshooting](docs/how-to/troubleshooting.md)
- [Contributor manual](AGENTS.md)

## License

[Apache-2.0](LICENSE)
