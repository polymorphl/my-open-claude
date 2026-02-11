//! Shared model info type (no dependencies on cache or API).

use serde::{Deserialize, Serialize};

/// Default context length when unknown (128k tokens).
pub const DEFAULT_CONTEXT_LENGTH: u64 = 128_000;

/// Lightweight model info for display and selection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    /// Maximum context window in tokens. Defaults to 128k when missing (backward compat).
    #[serde(default = "default_context_length")]
    pub context_length: u64,
}

fn default_context_length() -> u64 {
    DEFAULT_CONTEXT_LENGTH
}
