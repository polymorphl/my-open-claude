//! Centralized path helpers for config, cache, and data directories.

use std::path::PathBuf;

use crate::core::app;

/// Project directories (config, cache, data) from the standard platform locations.
pub fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("io", "polymorphl", app::NAME)
}

/// Override data dir for tests via env var. Set `TEST_DATA_DIR` before history operations.
#[cfg(test)]
fn test_data_dir_override() -> Option<PathBuf> {
    std::env::var("TEST_DATA_DIR").ok().map(PathBuf::from)
}

/// Config directory (~/.config/my-open-claude/).
pub fn config_dir() -> Option<PathBuf> {
    project_dirs().map(|d| d.config_dir().to_path_buf())
}

/// Cache directory (~/.cache/my-open-claude/).
pub fn cache_dir() -> Option<PathBuf> {
    project_dirs().map(|d| d.cache_dir().to_path_buf())
}

/// Data directory for conversations (~/.local/share/my-open-claude/conversations/).
/// In tests, set `TEST_DATA_DIR` env var to override.
pub fn data_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(p) = test_data_dir_override() {
        return Some(p);
    }
    project_dirs().map(|d| d.data_dir().join("conversations"))
}
