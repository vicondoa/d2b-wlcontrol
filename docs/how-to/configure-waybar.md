# Waybar integration

`d2b-wlcontrol waybar` is a continuous process. It emits and flushes one
newline-terminated JSON object per refresh, so the custom module must not set
`interval`.

```jsonc
"custom/d2b-wlcontrol": {
  "exec": "d2b-wlcontrol waybar",
  "return-type": "json",
  "restart-interval": 5,
  "signal": 8,
  "on-click": "d2b-wlcontrol open",
  "on-click-right": "d2b-wlcontrol action cycle-display",
  "on-click-middle": "d2b-wlcontrol action refresh",
  "tooltip": true
}
```

Generate this with `d2b-wlcontrol print-waybar-config`, or enable the host Home
Manager module.

## Output

Every update contains only:

- `text`: bounded configured icon/label plus running/visible VM count;
- `class`: a bounded array of stable CSS classes; and
- `tooltip`: VM status and public realm/workload launcher detail.

Workload targets, realm names, launcher names, and capability tokens never
become CSS classes. Unsafe-local adds only `unsafe-local`; a card mixing unsafe
and isolated workloads may also add `mixed-isolation`.

Stable classes:

| Class | Meaning |
| --- | --- |
| `all-stopped`, `partial-running`, `all-running` | VM aggregate state |
| `attention` | VM drift/error/hot mic or actionable workload unavailability |
| `daemon-down`, `auth-denied`, `stale` | connection/cache posture |
| `unsafe-local` | at least one public workload has no isolation |
| `mixed-isolation` | a realm card mixes unsafe and isolated/provider rows |

The tooltip uses each configured item's own name/icon and includes provider,
no-isolation, user-manager-lifetime, and availability remediation text.

## Styling

`d2b-wlcontrol print-css` prints the starter stylesheet. It imports
`/etc/d2b/ui-colors.css` and uses d2b's state colors. The Home Manager module
installs the same CSS and can append it to `programs.waybar.style`; override
`programs.d2b-wlcontrol.waybar.css` to preserve a custom stylesheet.

## Clicks and refresh

- left: toggle the control center;
- right: cycle compact/detail text;
- middle: request immediate refresh.

`signal = 8` pairs with the process's `SIGRTMIN+8` handler. Refreshes do not
overlap, and daemon-down polling uses bounded backoff.
