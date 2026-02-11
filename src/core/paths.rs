//! Centralized path helpers for config, cache, and data directories.

use std::path::PathBuf;

/// Project directories (config, cache, data) from the standard platform locations.
pub fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("io", "polymorphl", "my-open-claude")
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
pub fn data_dir() -> Option<PathBuf> {
    project_dirs().map(|d| d.data_dir().join("conversations"))
}
