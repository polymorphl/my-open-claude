//! Persistence of user preferences (e.g. last selected model) in ~/.config/my-open-claude/.

use std::fs;
use std::io;

use crate::core::paths;

/// Load the last used model ID from disk, if the file exists.
pub fn load_last_model() -> Option<String> {
    let path = paths::config_dir()?.join("last_model");
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Save the last used model ID to disk. Creates the config directory if needed.
pub fn save_last_model(model_id: &str) -> io::Result<()> {
    let dir = paths::config_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No config directory"))?;
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("last_model"), model_id)
}
