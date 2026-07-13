# niri / Wayland integration

The control center is a native Quickshell layer-shell surface. It requires no
niri window rule and makes no XWayland assumptions. `d2b-wlcontrol open`
toggles a draggable top-right popup; long realm/VM lists scroll inside the
bounded panel.

The popup starts unpinned. After it has received focus, moving focus elsewhere
closes it; use the keyboard-accessible pin in the top-right controls to keep
that invocation open. Pin state and dragged placement are process-local, so
closing and reopening restores the 24 px top-right placement with pinning off.
Drag empty header chrome to move the fixed-size card. Its full movement area is
the compositor-provided layer-shell work area, so Waybar's exclusive zone,
output size, and scale remain authoritative without a niri IPC dependency.
Escape and the close control always close the popup, including while pinned.

## Deterministic review render

From a live niri/Wayland session with Quickshell and the Material Symbols font
available, render dense mocked state through the production QML tree:

```bash
d2b-wlcontrol render-sample --output "$PWD/wlcontrol-panel.png"
```

The output path is required and its parent directory must exist. Render mode
does not query d2bd, enumerate USB devices, or dispatch actions. It waits for
layout and a rendered frame, fails after a bounded timeout, and verifies a
420×640 physical-pixel PNG with non-empty image payload below 5 MiB. Capture
dimensions are normalized across fractional output scales. Generated PNGs are
review artifacts and should remain untracked.

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
