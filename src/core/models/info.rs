//! Shared model info type (no dependencies on cache or API).

use serde::{Deserialize, Serialize};

/// Lightweight model info for display and selection.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
}
