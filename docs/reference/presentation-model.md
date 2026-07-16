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

Live source fragments and actions may be connected only through canonical
`d2b-client-toolkit` service clients after the owning service contracts are
available. Until then, the adapter returns disconnected state and rejects
daemon actions. It does not retain the removed JSON transport or infer a future
route, endpoint, notification, or desktop-action API.

Realm, VM, and state accents continue to come from the configured public d2b UI
color artifact. The neutral popup theme remains independently configurable and
does not depend on Stylix.
