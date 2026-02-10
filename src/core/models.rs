//! Fetch available models from OpenRouter (filtered by tool support).

use openrouter_rs::{OpenRouterClient, types::SupportedParameters};
use std::error::Error;
use std::io;

use crate::core::config::Config;
use crate::core::models_cache;

pub use crate::core::model_info::ModelInfo;

/// Filter models by query (case-insensitive match on id or name).
pub fn filter_models<'a>(models: &'a [ModelInfo], query: &str) -> Vec<&'a ModelInfo> {
    if query.is_empty() {
        return models.iter().collect();
    }
    let q = query.to_lowercase();
    models
        .iter()
        .filter(|m| m.id.to_lowercase().contains(&q) || m.name.to_lowercase().contains(&q))
        .collect()
}

/// Resolve model ID to display name. Uses cached models if available; otherwise returns the ID (slug).
pub fn resolve_model_display_name(model_id: &str) -> String {
    models_cache::load_cached_models()
        .and_then(|models| {
            models
                .into_iter()
                .find(|m| m.id == model_id)
                .map(|m| m.name)
        })
        .unwrap_or_else(|| model_id.to_string())
}

/// Fetch models that support tool calling, suitable for the agent.
/// Uses 24h cache; sorts alphabetically by name.
pub async fn fetch_models_with_tools(
    config: &Config,
) -> Result<Vec<ModelInfo>, Box<dyn Error + Send + Sync>> {
    if let Some(mut cached) = models_cache::load_cached_models() {
        cached.sort_by(|a, b| a.name.cmp(&b.name));
        return Ok(cached);
    }

    let client = OpenRouterClient::builder()
        .api_key(config.api_key())
        .build()?;

    let models = client
        .list_models_by_parameters(SupportedParameters::Tools)
        .await
        .map_err(|e| {
            let msg = e.to_string();
            Box::new(io::Error::new(io::ErrorKind::Other, msg)) as Box<dyn Error + Send + Sync>
        })?;

    let mut model_infos: Vec<ModelInfo> = models
        .into_iter()
        .map(|m| ModelInfo {
            id: m.id,
            name: m.name,
        })
        .collect();

    model_infos.sort_by(|a, b| a.name.cmp(&b.name));
    if let Err(e) = models_cache::save_models_to_cache(&model_infos) {
        eprintln!("Warning: failed to save models cache: {}", e);
    }
    Ok(model_infos)
}
