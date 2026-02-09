use std::env;
use std::process;

use async_openai::config::OpenAIConfig;

pub fn load() -> OpenAIConfig {
    let base_url = env::var("OPENROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

    let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
        eprintln!("OPENROUTER_API_KEY is not set");
        process::exit(1);
    });

    OpenAIConfig::new()
        .with_api_base(base_url)
        .with_api_key(api_key)
}
