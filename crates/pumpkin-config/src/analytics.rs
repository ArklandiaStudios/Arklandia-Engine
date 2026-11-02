use serde::{Deserialize, Serialize};

/// Server analytics. Not implemented yet: the switch stays off and nothing is collected or sent.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(default)]
pub struct AnalyticsConfig {
    /// Reserved for the future analytics feature.
    pub enabled: bool,
}

impl AnalyticsConfig {
    /// Validates the analytics configuration options.
    pub const fn validate(&self) {}
}
