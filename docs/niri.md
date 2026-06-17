# niri / Wayland integration

`nixling-wlcontrol` targets niri (and Wayland compositors generally)
natively. It makes **no XWayland assumptions** and uses:

- a Waybar custom module for the bar indicator; and
- a Quickshell layer-shell popup for the control surface.

## Popup behavior

`nixling-wlcontrol open` toggles a draggable top-right Quickshell popup:

- first invocation shows it;
- the next invocation hides it;
- the popup is a layer-shell surface, not a normal tiled window;
- drag the header/background to reposition it after opening;
- the popup fits its VM cards until it reaches about half the screen height,
  then uses a thin scrollbar for overflow; and
- no niri `window-rule` is required.

This matches Waybar click ergonomics: bind left-click to
`nixling-wlcontrol open`, click once to show controls, click again to
hide them.

## Theme

The popup uses the same Catppuccin-like color language as the shipped
Waybar CSS: dark base, green running/start, red stop, peach restart or
attention, teal USB, blue switch, and purple terminal.

If you replace the generated CSS with your own Waybar colors, keep the
same semantic mapping so the bar and popup still read as one UI.
