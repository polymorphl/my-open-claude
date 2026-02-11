//! Fetch credit balance from OpenRouter.

use openrouter_rs::api::credits::{CreditsData, get_credits};
use std::error::Error;

use crate::core::config::Config;

/// Fetch credit balance (total_credits, total_usage).
/// Requires Management API key; may fail with 401/403 for regular keys.
pub async fn fetch_credits(config: &Config) -> Result<CreditsData, Box<dyn Error + Send + Sync>> {
    Ok(get_credits(config.base_url(), config.api_key()).await?)
}
