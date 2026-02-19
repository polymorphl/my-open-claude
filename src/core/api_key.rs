//! API key storage: load and persist OPENROUTER_API_KEY in the config directory.
//!
//! The key is stored in a dedicated file with restrictive permissions (0o600 on Unix).

use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::core::paths;

/// Errors when loading or storing the API key.
#[derive(Debug, thiserror::Error)]
pub enum ApiKeyError {
    #[error("No config directory available")]
    NoConfigDir,
    #[error("Failed to store API key: {0}")]
    Io(#[from] io::Error),
}

/// Path to the API key file in the config directory.
pub fn credentials_path() -> Option<PathBuf> {
    paths::config_dir().map(|d| d.join("api-key"))
}

/// Load the API key from the config directory.
/// Returns `None` if the file is absent, empty, or unreadable.
pub fn load_api_key() -> Option<String> {
    let path = credentials_path()?;
    let content = fs::read_to_string(&path).ok()?;
    let key = content.trim().to_string();
    if key.is_empty() { None } else { Some(key) }
}

/// Store the API key in the config directory.
/// Creates the config dir if needed. On Unix, sets file permissions to 0o600.
pub fn store_api_key(key: &str) -> Result<(), ApiKeyError> {
    let path = credentials_path().ok_or(ApiKeyError::NoConfigDir)?;
    let dir = path.parent().ok_or_else(|| {
        ApiKeyError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid credentials path",
        ))
    })?;
    fs::create_dir_all(dir)?;

    let mut file = fs::File::create(&path)?;
    let trimmed = key.trim();
    file.write_all(trimmed.as_bytes())?;
    file.write_all(b"\n")?;

    #[cfg(unix)]
    {
        let mut perms = file.metadata()?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{load_api_key, store_api_key};

    #[test]
    fn roundtrip_store_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path();
        unsafe { std::env::set_var("TEST_CONFIG_DIR", config_dir) };

        store_api_key("sk-test-key-123").unwrap();
        let loaded = load_api_key();
        assert_eq!(loaded.as_deref(), Some("sk-test-key-123"));

        unsafe { std::env::remove_var("TEST_CONFIG_DIR") };
    }
}
