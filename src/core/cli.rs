//! CLI-only commands: config info, models list, history list.
//!
//! These run without opening the TUI and produce plain text output.

use std::env;
use std::io::{self, Read};

use crate::core::api_key;
use crate::core::config::{self, ConfigError};
use crate::core::history;
use crate::core::models;
use crate::core::paths;
use crate::core::persistence;

/// Run the `config` command: display paths, model, and API key status.
pub fn run_config() {
    let config_dir = paths::config_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "—".to_string());
    let cache_dir = paths::cache_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "—".to_string());
    let data_dir = paths::data_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "—".to_string());

    let (model, model_source, api_key_status) = match config::load() {
        Ok(c) => (c.model_id, model_source(), "set ✓"),
        Err(ConfigError::MissingApiKey) => {
            let (id, src) = fallback_model();
            (id, src, "not set")
        }
    };

    println!("Config:        {}", config_dir);
    println!("Cache:        {}", cache_dir);
    println!("Conversations: {}", data_dir);
    println!("Model:        {} ({})", model, model_source);
    println!("API key:      {}", api_key_status);
}

/// Run the `config set-api-key` command: store API key in config directory.
pub fn run_config_set_api_key(api_key: Option<String>) {
    let key = match api_key {
        Some(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => {
            let mut buf = String::new();
            if let Err(e) = io::stdin().read_to_string(&mut buf) {
                eprintln!("Error reading from stdin: {}", e);
                std::process::exit(1);
            }
            let trimmed = buf.trim().to_string();
            if trimmed.is_empty() {
                eprintln!("Error: no API key provided");
                std::process::exit(1);
            }
            trimmed
        }
    };

    match api_key::store_api_key(&key) {
        Ok(()) => {
            let path = api_key::credentials_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "config directory".to_string());
            println!("API key saved to {}", path);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn model_source() -> &'static str {
    if persistence::load_last_model().is_some() {
        "from last_model"
    } else if env::var("OPENROUTER_MODEL")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some()
    {
        "from OPENROUTER_MODEL"
    } else {
        "default"
    }
}

fn fallback_model() -> (String, &'static str) {
    if let Some(id) = persistence::load_last_model() {
        return (id, "from last_model");
    }
    if let Ok(id) = env::var("OPENROUTER_MODEL")
        && !id.is_empty()
    {
        return (id, "from OPENROUTER_MODEL");
    }
    ("anthropic/claude-haiku-4.5".to_string(), "default")
}

/// Format context length as human-readable (e.g. "128k", "1M").
fn format_context(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{}k", n / 1_000)
    } else {
        n.to_string()
    }
}

/// Run the `models` command: list available models (tool-capable) from cache or API.
pub async fn run_models(config: &crate::core::config::Config, query: Option<&str>) {
    let models = match models::fetch_models_with_tools(config).await {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let filtered: Vec<_> = match query {
        Some(q) => models::filter_models(&models, q),
        None => models.iter().collect(),
    };

    if filtered.is_empty() {
        println!("No models found.");
        return;
    }

    let id_w = filtered
        .iter()
        .map(|m| m.id.len())
        .max()
        .unwrap_or(20)
        .max(20);
    let name_w = filtered
        .iter()
        .map(|m| m.name.len())
        .max()
        .unwrap_or(30)
        .max(30);

    println!("{:<id_w$}  {:<name_w$}  {:>6}", "ID", "Name", "Context");
    println!("{}  {}  ------", "-".repeat(id_w), "-".repeat(name_w));

    for m in &filtered {
        let ctx = format_context(m.context_length);
        println!("{:<id_w$}  {:<name_w$}  {:>6}", m.id, m.name, ctx);
    }

    println!("\n{} model(s) listed", filtered.len());
}

/// Run the `history list` command: list conversations with optional limit.
pub fn run_history_list(limit: Option<usize>) {
    let convs = match history::list_conversations() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let take = limit.unwrap_or(convs.len());
    for c in convs.into_iter().take(take) {
        let created = format_timestamp(c.created_at);
        let updated = format_timestamp(c.updated_at);
        println!("{}\t{}\t{}\t{}", c.id, c.title, created, updated);
    }
}

fn format_timestamp(secs: u64) -> String {
    use chrono::{TimeZone, Utc};
    let dt = Utc.timestamp_opt(secs as i64, 0).single();
    dt.map(|d| d.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| secs.to_string())
}
