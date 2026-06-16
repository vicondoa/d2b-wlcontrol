//! GTK4/libadwaita control center.
//!
//! Owning wave: **Wave 2 — GTK control center agent**. Wave 0 provides the
//! public entry point ([`open`]) so `wlcontrol-cli` can wire `nixling-wlcontrol
//! open` today; the Wave 2 agent adds the `gtk4` + `libadwaita` dependencies
//! and implements VM cards, auth-aware action gating, and async action workers.

use wlcontrol_core::error::{WlError, WlResult};
use wlcontrol_core::Config;

/// Open (or focus) the control center window.
///
/// Wave 0 stub: returns a typed error until the Wave 2 GTK implementation
/// lands. Single-instance open/focus semantics are part of the Wave 2 scope.
pub fn open(_config: &Config) -> WlResult<()> {
    Err(WlError::Config(
        "the GTK control center is implemented in Wave 2".to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_is_stubbed_until_wave_2() {
        assert!(open(&Config::default()).is_err());
    }
}
