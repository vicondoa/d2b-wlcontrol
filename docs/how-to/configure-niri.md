# niri / Wayland integration

The control center is a native Quickshell layer-shell surface. It requires no
niri window rule and makes no XWayland assumptions. `d2b-wlcontrol open`
toggles a draggable top-right popup; long realm/VM lists scroll inside the
bounded panel.

## Realm presentation

Realm cards use d2b accent metadata. The popup's neutral background/text palette
comes from `[theme]`, while realm rails, VM borders, and state accents continue
to come from d2b's public UI color artifact. Missing or malformed accent data
removes the affected accent instead of inventing a policy color.

Unsafe-local windows launched by d2b are expected to pass through
`d2b-wayland-proxy` and receive their realm identity rail. Wlcontrol displays the
same realm accent and a no-isolation warning, but the color/rail is presentation
metadata—not containment or authorization.

## Launch paths

- Configured graphical `exec` items are submitted to d2b configured launch.
  Provider status must say the Wayland proxy is available; there is no direct
  compositor fallback.
- Configured `shell` items invoke wlterm with the canonical workload target.
  Wlterm owns the persistent shell and terminal-window proxy integration.
- Existing local VM terminal controls remain guest-control operations and are
  not used for unsafe-local workloads.

If the graphical user manager, Wayland session, or proxy is unavailable, the
row remains visible but disabled with remediation.
