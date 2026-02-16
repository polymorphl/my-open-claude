use std::env;

use async_openai::config::OpenAIConfig;

use crate::core::persistence;

/// Represents the configuration for the AI chat application.
///
/// # Fields
/// * `openai_config`: Configuration for OpenAI/OpenRouter API interactions
/// * `model_id`: ID of the selected AI model
/// * `base_url`: Base URL for the AI service API
/// * `api_key`: Authentication API key for the service
/// * `max_conversations`: Maximum number of conversations to retain
/// * `show_timestamps`: Whether to show timestamps next to messages in the TUI
#[derive(Debug, Clone)]
pub struct Config {
    pub openai_config: OpenAIConfig,
    pub model_id: String,
    pub base_url: String,
    pub api_key: String,
    pub max_conversations: u32,
    pub show_timestamps: bool,
}

/// Errors that can occur during configuration loading.
#[derive(Debug)]
pub enum ConfigError {
    /// Indicates that the required API key is missing from environment variables
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
    /// Returns the configured API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Returns the base URL for the AI service.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

/// Default AI model to use if no model is specified
const DEFAULT_MODEL: &str = "anthropic/claude-haiku-4.5";

/// Load configuration from environment variables and persistent storage.
///
/// # Configuration Resolution Order
/// 1. Last used model from persistent storage
/// 2. OPENROUTER_MODEL environment variable
/// 3. Default model
///
/// # Environment Variables
/// * `OPENROUTER_BASE_URL`: Custom base URL for AI service (optional)
/// * `OPENROUTER_API_KEY`: Required API key
/// * `OPENROUTER_MODEL`: Preferred model (optional)
/// * `MY_OPEN_CLAUDE_MAX_CONVERSATIONS`: Maximum conversations to retain (optional)
/// * `MY_OPEN_CLAUDE_SHOW_TIMESTAMPS`: Set to 1 or true to show timestamps next to messages (optional)
///
/// # Returns
/// A `Result` containing the loaded `Config` or a `ConfigError`
pub fn load() -> Result<Config, ConfigError> {
    // Determine base URL, defaulting to OpenRouter's API
    let base_url = env::var("OPENROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

    // Require API key
    let api_key = env::var("OPENROUTER_API_KEY").map_err(|_| ConfigError::MissingApiKey)?;

    // Resolve model selection
    let model_id = persistence::load_last_model()
        .or_else(|| env::var("OPENROUTER_MODEL").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());

    // Configure max conversations, with a sensible default
    const DEFAULT_MAX_CONVERSATIONS: u32 = 50;
    let max_conversations = env::var("MY_OPEN_CLAUDE_MAX_CONVERSATIONS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(DEFAULT_MAX_CONVERSATIONS);

    let show_timestamps = env::var("MY_OPEN_CLAUDE_SHOW_TIMESTAMPS")
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    // Create OpenAI/OpenRouter configuration
    let openai_config = OpenAIConfig::new()
        .with_api_base(&base_url)
        .with_api_key(&api_key);

    Ok(Config {
        openai_config,
        model_id,
        base_url,
        api_key,
        max_conversations,
        show_timestamps,
    })
}
