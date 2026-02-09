use std::env;

use async_openai::config::OpenAIConfig;

#[derive(Debug, Clone)]
pub struct Config {
    pub openai_config: OpenAIConfig,
    pub model_id: String,
}

#[derive(Debug)]
pub enum ConfigError {
    MissingApiKey,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::MissingApiKey => write!(f, "OPENROUTER_API_KEY is not set"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Load configuration from environment. Returns an error if API key is missing.
pub fn load() -> Result<Config, ConfigError> {
    let base_url = env::var("OPENROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

    let api_key = env::var("OPENROUTER_API_KEY").map_err(|_| ConfigError::MissingApiKey)?;

    let model_id =
        env::var("OPENROUTER_MODEL").unwrap_or_else(|_| "anthropic/claude-haiku-4.5".to_string());

    let openai_config = OpenAIConfig::new()
        .with_api_base(base_url)
        .with_api_key(api_key);

    Ok(Config {
        openai_config,
        model_id,
    })
}
