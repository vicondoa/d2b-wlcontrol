# Changelog

All notable changes to `d2b-wlcontrol` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project follows
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-07-11

### Added

- Added public workload inventory/status support with canonical targets,
  provider kind, structured execution/isolation posture, availability,
  forward-compatible capability tokens, and generic configured launcher items.
- Added first-class unsafe-local realm rows with explicit no-isolation and
  user-manager-lifetime warnings plus actionable helper, graphical-session,
  Wayland, and proxy remediation.
- Added generic exec dispatch through `d2b launch <target> --item <id>` and
  persistent-shell dispatch through configurable wlterm argv using the canonical
  target.
- Added a host `homeManagerModules.default` that installs wlcontrol, renders its
  configuration, injects the self-looping Waybar module and CSS, preserves
  placement/click/icon/label/module overrides, and supports launcher-item
  presentation overrides.
- Added fixture, fake-daemon, reducer/planner, view-model/QML, Waybar golden,
  and Nix evaluation coverage for unsafe/mixed cards, first-class local VMs,
  generic exec/shell items, unknown capabilities, launch errors, and readiness
  remediation.

### Changed

- Replaced private launcher-artifact consumption and VM-shaped realm launcher
  assumptions with d2b-toolkit 0.2 public workload contracts.
- Realm cards now render every configured item from its item-owned name and icon;
  browsers and observability tools are ordinary exec items.
- Unsafe-local rows now omit VM lifecycle, build/boot/switch, store, USB, audio,
  VM terminal, quick-launch, and arbitrary guest-exec controls.
- Waybar retains its continuous newline-JSON contract and bounded labels/classes;
  unsafe posture can add only stable `unsafe-local` and `mixed-isolation`
  classes.
- Realm, VM, and state accents continue to use d2b UI metadata while the neutral
  popup palette remains independently configurable and Stylix-agnostic.
- Updated package, workspace, flake, and toolkit integration versions to 0.2.0.
- Public-socket receive handling rejects oversized `SOCK_SEQPACKET` packets
  before frame decoding instead of accepting a truncated packet, and retries
  interrupted packet reads and writes.

### Security

- Removed all launcher reads from root-owned/private d2b files and kept workload
  operations on the authenticated public socket or exact official CLI/wlterm
  argv boundaries.
- Unsafe-local presentation now states that identity rails are not isolation and
  refuses misleading VM-shaped controls.
