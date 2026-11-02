use serde::{Deserialize, Serialize};

/// Operator (OP) settings.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct OperatorsConfig {
    /// Whether operators exist. When false, `/op` and `/deop` are disabled and nobody
    /// receives operator rights, whatever the operator list contains.
    pub enabled: bool,
}

impl Default for OperatorsConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}
