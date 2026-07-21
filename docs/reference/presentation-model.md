# Presentation model

`d2b-wlcontrol` owns its user-interface state, not a d2b protocol.

The repository-local `WlState`, VM, realm, availability, action-planning, and
reducer-input types exist to keep Waybar and Quickshell rendering deterministic.
Their Serde shapes are local CLI/UI compatibility surfaces. They are not d2b
requests, responses, session records, handles, framing, or service bindings.

The reducer accepts normalized, optional source fragments and degrades missing
data to disconnected, unknown, or unavailable presentation state. Unit tests
exercise inventory conflicts, missing status, authorization, unsafe-local
posture, launcher overrides, audio errors, color selection, and stable Waybar
output without a daemon or copied protocol fixture.

Live local-VM fragments come from the canonical toolkit's authenticated
`DaemonClient` inspection projection. The adapter preserves typed pagination
and read-model consistency, normalizes only VM runtimes into VM presentation
rows, and maps only the frozen start/stop/restart lifecycle methods. A
successfully authenticated session proves launcher read posture but not admin
authority, so admin controls remain disabled until d2b exposes a canonical
caller-role projection.

User/Shell/Notify/Wayland service kinds are available as canonical types, but
their live endpoint and route acquisition remains runtime-owned. Wlcontrol
therefore does not instantiate desktop observer/action flows, translate their
DTOs, or fall back to a legacy socket, file, CLI callback, or direct compositor.

Realm, VM, and state accents continue to come from the configured public d2b UI
color artifact. The neutral popup theme remains independently configurable and
does not depend on Stylix. Non-VM projections are never normalized into VM
rows, so VM borders and controls cannot be attached to unsafe-local apps.
