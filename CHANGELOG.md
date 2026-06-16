# Changelog

All notable changes to `nixling-wlcontrol` are documented here. The
format follows [Keep a Changelog](https://keepachangelog.com/) and the
project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **Workspace and contract.** Rust workspace with `wlcontrol-core`
  (domain model, config, reducer, action planner), `wlcontrol-nixling`
  (public-socket client), `wlcontrol-waybar` (custom-module renderer),
  `wlcontrol-ui` (GTK4/libadwaita control center), and `wlcontrol-cli`
  (the `nixling-wlcontrol` binary).
- **Live nixlingd client.** Direct public-socket client speaking the
  non-abstract `SOCK_SEQPACKET` protocol: hello/version negotiation,
  4-byte little-endian length-prefixed JSON framing, typed responses,
  and translation of `auth status` / `list` / `status` / `usb probe`
  into a reduced control-surface state. A configured broker socket path
  is refused, and a mid-refresh failure degrades to daemon-down rather
  than reporting a false-healthy view.
- **Reduced state model.** Source-precedence reducer
  (`list` -> `status` -> `usb probe` -> `auth status`) with net-VM
  detection, favorites ordering, hidden-VM filtering, and
  inconsistency -> attention mapping.
- **Waybar module.** Continuous custom JSON module with compact and
  detail display modes, state-driven CSS classes, a rich per-VM tooltip
  (env, state, pending-restart, USB ownership), signal-driven refresh
  (`SIGRTMIN+8`), non-overlapping refresh, daemon-down backoff, and
  persisted display mode.
- **GTK control center.** Single-instance libadwaita application
  (app-id `dev.vicondoa.NixlingWlControl`) with per-env VM cards,
  auth-gated action controls (start/stop/restart/switch, launch
  terminal, USB attach/detach, store verify), off-main-thread socket
  dispatch with toasts, destructive-action confirmations, and
  daemon-down/auth-denied recovery pages. Audio controls render disabled
  until nixling exposes a daemon-native audio control plane.
- **Safety model.** Public socket only (never the broker socket), no
  `sudo`, no nixling state-file mutation, argv-only command execution,
  and authorization derived from `nixling auth status`.
- **Packaging and docs.** Nix flake (package/app/devShell with the GTK
  closure), CI gate, starter Waybar config + CSS + niri window rule,
  `AGENTS.md`, and the configuration / controls / Waybar / niri /
  security documentation set.
