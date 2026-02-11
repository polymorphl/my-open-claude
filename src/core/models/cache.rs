//! 24h cache for OpenRouter models list.

use super::info::ModelInfo;
use crate::core::paths;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

#[derive(Serialize, Deserialize)]
struct CachedModels {
    fetched_at: u64,
    models: Vec<ModelInfo>,
}

fn cache_path() -> Option<std::path::PathBuf> {
    paths::cache_dir().map(|d| d.join("models.json"))
}

/// Load cached models if fresh (< 24h). Returns None if cache miss or expired.
pub fn load_cached_models() -> Option<Vec<ModelInfo>> {
    let path = cache_path()?;
    let data = fs::read_to_string(path).ok()?;
    let cached: CachedModels = serde_json::from_str(&data).ok()?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs();
    let age_secs = now.saturating_sub(cached.fetched_at);
    if age_secs < CACHE_TTL.as_secs() {
        Some(cached.models)
    } else {
        None
    }
}

/// Save models to cache.
pub fn save_models_to_cache(models: &[ModelInfo]) -> io::Result<()> {
    let path = cache_path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No cache dir"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
        .as_secs();
    let cached = CachedModels {
        fetched_at: now,
        models: models.to_vec(),
    };
    fs::write(path, serde_json::to_string_pretty(&cached).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, e)
    })?)
}
