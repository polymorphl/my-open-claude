//! Persistence of user preferences (e.g. last selected model) in ~/.config/my-open-claude/.

use std::fs;
use std::io;
use std::path::PathBuf;

fn config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("io", "polymorphl", "my-open-claude")
        .map(|d| d.config_dir().to_path_buf())
}

/// Load the last used model ID from disk, if the file exists.
pub fn load_last_model() -> Option<String> {
    let path = config_dir()?.join("last_model");
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Save the last used model ID to disk. Creates the config directory if needed.
pub fn save_last_model(model_id: &str) -> io::Result<()> {
    let dir = config_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No config directory"))?;
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("last_model"), model_id)
}
