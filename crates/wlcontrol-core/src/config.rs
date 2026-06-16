//! User configuration for nixling-wlcontrol.
//!
//! Owning wave: **Wave 1 — Core model agent**. Wave 0 ships a minimal,
//! compiling skeleton with sane defaults and the on-disk location contract.
//! The Wave 1 agent fleshes out validation, terminal-argv parsing rules,
//! favorites/ordering, and the full option surface described in the plan.

use serde::{Deserialize, Serialize};

/// Default config file location: `${XDG_CONFIG_HOME:-~/.config}/nixling-wlcontrol/config.toml`.
pub const CONFIG_RELATIVE_PATH: &str = "nixling-wlcontrol/config.toml";

/// Top-level configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct Config {
    /// Path to the nixling public socket.
    pub public_socket: String,
    /// Refresh cadence in milliseconds for the Waybar loop.
    pub refresh_interval_ms: u64,
    /// Per-operation timeout in milliseconds.
    pub command_timeout_ms: u64,
    /// Hide framework net VMs from the compact surfaces.
    pub hide_net_vms: bool,
    /// Show the pending-restart marker.
    pub show_pending_restart: bool,
    /// Terminal launch configuration.
    pub terminal: TerminalConfig,
}

/// Terminal launch configuration. The terminal command is always an argv
/// vector; no shell string interpolation is performed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct TerminalConfig {
    /// argv prefix used to spawn a terminal, e.g. `["foot", "--"]`.
    pub argv: Vec<String>,
    /// Guest shell to run inside the VM, e.g. `bash`.
    pub guest_shell: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            public_socket: "/run/nixling/public.sock".to_owned(),
            refresh_interval_ms: 2500,
            command_timeout_ms: 4000,
            hide_net_vms: true,
            show_pending_restart: true,
            terminal: TerminalConfig::default(),
        }
    }
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            argv: vec!["foot".to_owned(), "--".to_owned()],
            guest_shell: "bash".to_owned(),
        }
    }
}

impl Config {
    /// Parse a configuration from a TOML string.
    pub fn from_toml(s: &str) -> crate::error::WlResult<Self> {
        toml::from_str(s).map_err(|e| crate::error::WlError::Config(e.to_string()))
    }

    /// Resolve the default config path under `$XDG_CONFIG_HOME`.
    pub fn default_path() -> Option<std::path::PathBuf> {
        directories::BaseDirs::new().map(|d| d.config_dir().join(CONFIG_RELATIVE_PATH))
    }

    /// Load configuration from the default path, falling back to built-in
    /// defaults when the file is absent. A present-but-malformed file is an
    /// error so the operator notices rather than silently getting defaults.
    pub fn load() -> crate::error::WlResult<Self> {
        match Self::default_path() {
            Some(path) if path.exists() => {
                let text = std::fs::read_to_string(&path)?;
                Self::from_toml(&text)
            }
            _ => Ok(Self::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_sane() {
        let c = Config::default();
        assert_eq!(c.public_socket, "/run/nixling/public.sock");
        assert!(c.hide_net_vms);
        assert_eq!(c.terminal.guest_shell, "bash");
    }

    #[test]
    fn empty_toml_uses_defaults() {
        let c = Config::from_toml("").expect("parse empty");
        assert_eq!(c, Config::default());
    }
}
