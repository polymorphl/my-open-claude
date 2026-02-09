use async_openai::Client;
use serde_json::{Value, json};

use crate::features::tools;

pub async fn chat(prompt: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let config = crate::features::config::load();
    let client = Client::with_config(config);

    let response: Value = client
        .chat()
        .create_byot(json!({
            "model": "anthropic/claude-haiku-4.5",
            "messages": [{"role": "user", "content": prompt}],
            "tool_choice": "auto",
            "tools": tools::definitions(),
        }))
        .await?;

    Ok(response)
}
