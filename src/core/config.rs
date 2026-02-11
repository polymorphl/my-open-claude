use std::env;

use async_openai::config::OpenAIConfig;

use crate::core::persistence;

#[derive(Debug, Clone)]
pub struct Config {
    pub openai_config: OpenAIConfig,
    pub model_id: String,
    #[allow(dead_code)] // for future openrouter base_url / credits integration
    pub base_url: String,
    pub api_key: String,
    /// Max number of conversations to keep. Prune older ones when exceeded.
    pub max_conversations: u32,
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

impl Config {
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    #[allow(dead_code)]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

const DEFAULT_MODEL: &str = "anthropic/claude-haiku-4.5";

/// Load configuration from environment. Returns an error if API key is missing.
/// Model resolution order: persisted last_model > OPENROUTER_MODEL > default.
pub fn load() -> Result<Config, ConfigError> {
    let base_url = env::var("OPENROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

    let api_key = env::var("OPENROUTER_API_KEY").map_err(|_| ConfigError::MissingApiKey)?;

    let model_id = persistence::load_last_model()
        .or_else(|| env::var("OPENROUTER_MODEL").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());

    const DEFAULT_MAX_CONVERSATIONS: u32 = 50;
    let max_conversations = env::var("MY_OPEN_CLAUDE_MAX_CONVERSATIONS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(DEFAULT_MAX_CONVERSATIONS);

    let openai_config = OpenAIConfig::new()
        .with_api_base(&base_url)
        .with_api_key(&api_key);

    Ok(Config {
        openai_config,
        model_id,
        base_url,
        api_key,
        max_conversations,
    })
}
