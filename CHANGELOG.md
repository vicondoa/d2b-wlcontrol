# Changelog

All notable changes to `nixling-wlcontrol` are documented here. The
format follows [Keep a Changelog](https://keepachangelog.com/) and the
project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Project bootstrap: Rust workspace with `wlcontrol-core`,
  `wlcontrol-nixling`, `wlcontrol-waybar`, `wlcontrol-ui`, and
  `wlcontrol-cli` crates.
- Frozen cross-crate domain contract (`wlcontrol-core::model`): reduced
  VM/auth/USB state model, action kinds, availability gating, and
  argv-only action planning.
- Baseline state reducer with source precedence
  (`list` → `status` → `usb probe` → `auth status`).
- Baseline Waybar custom-module renderer (compact text, state classes,
  per-VM tooltip).
- nixlingd public-socket framing primitives (4-byte little-endian
  length prefix, 1 MiB cap) and the public client surface.
- `nixling-wlcontrol` CLI skeleton: `waybar`, `open`, `status-json`,
  `action`, `print-waybar-config`, `print-css`.
- Nix flake (package / app / devShell), CI workflow, starter Waybar
  config + CSS + niri window rule.
- `AGENTS.md` operating manual (adapted from nixling) and the docs
  skeleton.
