//! Fetch available models from OpenRouter (filtered by tool support).

use openrouter_rs::{OpenRouterClient, types::SupportedParameters};
use std::error::Error;
use std::io;

use crate::core::config::Config;
use crate::core::util;

use super::cache;
use super::info::ModelInfo;

/// Filter models by query (case-insensitive match on id or name).
pub fn filter_models<'a>(models: &'a [ModelInfo], query: &str) -> Vec<&'a ModelInfo> {
    util::filter_by_query(models, query, |m| (m.id.as_str(), m.name.as_str()))
}

/// Resolve model ID to display name. Uses cached models if available; otherwise returns the ID (slug).
pub fn resolve_model_display_name(model_id: &str) -> String {
    cache::load_cached_models()
        .and_then(|models| {
            models
                .into_iter()
                .find(|m| m.id == model_id)
                .map(|m| m.name)
        })
        .unwrap_or_else(|| model_id.to_string())
}

/// Resolve model ID to its context length. Falls back to default if not found.
pub fn resolve_context_length(model_id: &str) -> u64 {
    cache::load_cached_models()
        .and_then(|models| {
            models
                .into_iter()
                .find(|m| m.id == model_id)
                .map(|m| m.context_length)
        })
        .unwrap_or(super::info::DEFAULT_CONTEXT_LENGTH)
}

/// Fetch models that support tool calling, suitable for the agent.
/// Uses 24h cache; sorts alphabetically by name.
pub async fn fetch_models_with_tools(
    config: &Config,
) -> Result<Vec<ModelInfo>, Box<dyn Error + Send + Sync>> {
    if let Some(mut cached) = cache::load_cached_models() {
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
            Box::new(io::Error::other(msg)) as Box<dyn Error + Send + Sync>
        })?;

    let mut model_infos: Vec<ModelInfo> = models
        .into_iter()
        .map(|m| {
            let context_length = if m.context_length > 0.0 {
                m.context_length as u64
            } else {
                super::info::DEFAULT_CONTEXT_LENGTH
            };
            ModelInfo {
                id: m.id,
                name: m.name,
                context_length,
            }
        })
        .collect();

    model_infos.sort_by(|a, b| a.name.cmp(&b.name));
    // Cache save failure is non-fatal: we still return the freshly fetched models.
    if let Err(e) = cache::save_models_to_cache(&model_infos) {
        eprintln!("Warning: failed to save models cache: {}", e);
    }
    Ok(model_infos)
}
