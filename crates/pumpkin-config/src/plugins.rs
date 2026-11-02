use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Plugin system configuration.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct PluginsConfig {
    /// Whether the plugin system is enabled. If false, no plugins will be loaded.
    pub enabled: bool,
    /// Whether to watch the plugins directory and automatically hot-reload modified plugins.
    pub hot_reload: bool,
    /// List of permissions globally approved for all plugins.
    pub allowed_permissions: Vec<String>,
    /// List of permissions globally blocked for all plugins.
    pub blocked_permissions: Vec<String>,
    /// Whether host environment variables are inherited by default.
    pub inherit_env: bool,
    /// Whether network sockets are restricted to localhost by default.
    pub loopback_only: bool,
    /// Optional global maximum memory limit in megabytes per plugin instance.
    pub max_memory_mb: Option<u64>,
    /// Number of threads of the dedicated pool used by bulk world edits requested by plugins
    /// (`get_block_state_ids` / `set_block_state_ids`). These threads are separate from the
    /// tick thread, the async runtime and the global rayon pool.
    /// `0` (default) means automatic: `max(1, cpu cores - 2)`.
    pub world_edit_threads: usize,
    /// Per-plugin configuration and overrides, keyed by plugin name.
    pub overrides: HashMap<String, PluginOverride>,
}

impl Default for PluginsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hot_reload: false,
            allowed_permissions: Vec::new(),
            blocked_permissions: Vec::new(),
            inherit_env: false,
            loopback_only: false,
            max_memory_mb: None,
            world_edit_threads: 0,
            overrides: HashMap::new(),
        }
    }
}

/// Per-plugin loading and permission overrides.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct PluginOverride {
    /// Whether this plugin is enabled.
    pub enabled: bool,
    /// Optional per-plugin memory limit in megabytes.
    pub max_memory_mb: Option<u64>,
    /// Permissions pre-approved for this plugin.
    pub allowed_permissions: Vec<String>,
    /// Permissions denied for this plugin.
    pub blocked_permissions: Vec<String>,
    /// Whether network access is limited to loopback for this plugin.
    pub loopback_only: Option<bool>,
    /// Environment variables passed to this plugin.
    pub environment: HashMap<String, String>,
}

impl Default for PluginOverride {
    fn default() -> Self {
        Self {
            enabled: true,
            max_memory_mb: None,
            allowed_permissions: Vec::new(),
            blocked_permissions: Vec::new(),
            loopback_only: None,
            environment: HashMap::new(),
        }
    }
}
