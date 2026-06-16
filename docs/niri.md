# niri / Wayland integration

`nixling-wlcontrol` targets niri (and Wayland compositors generally)
natively. It makes **no XWayland assumptions** and uses a Waybar custom
module for the panel rather than a legacy tray/AppIndicator, which is
inconsistent on pure Wayland.

## Control-center window rule

The GTK control center uses a stable application id
`dev.vicondoa.NixlingWlControl`. To make it open as a tidy floating
window in niri, add the rule from
[`data/niri-window-rule.kdl`](../data/niri-window-rule.kdl) to your niri
config:

```kdl
window-rule {
    match app-id="dev.vicondoa.NixlingWlControl"

    open-floating true
    default-column-width { fixed 520; }
    default-window-height { fixed 640; }
}
```

## Single instance

`nixling-wlcontrol open` opens or focuses a single control-center
instance, so repeated clicks on the Waybar module never spawn duplicate
windows.

> Owning wave: Wave 2 implements the GTK application (app-id,
> single-instance open/focus). Until then `open` returns a typed
> "implemented in Wave 2" error.
