# Troubleshooting

## No workload cards

Run `d2b-wlcontrol status-json` and inspect `realmGroups`. An empty list means
the connected daemon did not return public workload inventory (or no workloads
are configured). Upgrade d2b/toolkit together; do not grant wlcontrol access to
private launcher files.

## Helper unavailable or stale

Follow the remediation shown in the row:

- unavailable: enable and start the d2b unsafe-local user service;
- stale: restart that user service so it matches d2bd.

Wlcontrol does not contact the helper directly and will not fall back to root or
the broker.

## User manager unavailable

Unsafe-local launch requires a normal graphical PAM login with a functioning
`systemd --user` and user D-Bus session. Re-enter the graphical session and
verify the user manager is active. Do not set a guessed D-Bus address in
wlcontrol.

## Wayland or proxy unavailable

Restore the graphical Wayland session or restart d2b desktop user services.
Graphical configured launch intentionally has no direct-host-compositor
fallback.

## Shell item does not open

Ensure `d2b-wlterm` is installed on `PATH`, or configure:

```toml
[terminal]
wlterm_argv = ["/absolute/path/to/d2b-wlterm", "open"]
```

The final argv is `... <canonical-target> <item-id>`. A shell item is not VM
guest exec.

## Exec item reports a launch error

Run the same public command to see d2b's typed error:

```bash
d2b launch tools.host.d2b --item firefox
```

Do not replace it with the private command declared by the workload.

## Unsafe-local controls are missing

This is intentional. Unsafe-local rows expose configured items, status, and
warnings only. VM lifecycle, build/switch, store, USB, audio, and guest-exec
controls would falsely imply a VM boundary.

## Waybar does not update

The module is self-looping. Remove `interval`, retain `restart-interval = 5`,
and keep `signal = 8`. Verify each output line from
`d2b-wlcontrol waybar` is one complete JSON object.

## Colors are absent

Check the configured public UI color artifact. Invalid/missing metadata removes
the accent by design. The neutral `[theme]` palette remains usable and can be
set independently of Stylix.
