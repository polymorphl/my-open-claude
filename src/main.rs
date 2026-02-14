//! # My Open Claude - AI Assistant CLI
//!
//! This is the main entry point for the My Open Claude application,
//! which provides an interactive CLI and TUI for AI-powered chat interactions.
//!
//! ## Features
//! - Single prompt mode with `-p` or `--prompt`
//! - Interactive terminal UI (TUI) for ongoing chat sessions
//! - Configuration management
//! - Error handling and graceful exits

mod core;
mod tui;

use clap::Parser;
use dotenv::dotenv;

/// Command-line arguments for the application
///
/// Supports two primary modes:
/// 1. Single prompt mode (with `-p`)
/// 2. Interactive TUI mode (default)
#[derive(Parser)]
#[command(
    author,
    version,
    about = "An AI Assistant CLI powered by open-source models"
)]
struct Args {
    /// Send a single prompt then exit (without opening the TUI)
    #[arg(
        short = 'p',
        long,
        help = "Provide a prompt to get an immediate AI response"
    )]
    prompt: Option<String>,
}

/// Main application entry point
///
/// Handles:
/// - Environment configuration via dotenv
/// - CLI argument parsing
/// - Single prompt processing
/// - Interactive TUI launch
///
/// # Errors
/// Returns an error if configuration loading fails or TUI encounters issues
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv().ok();

    // Initialize logging (warn level by default; use RUST_LOG=debug for verbose)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .try_init()
        .ok();

    // Parse command-line arguments
    let args = Args::parse();

    // Load application configuration
    let config = core::config::load()?;

    // Handle single prompt mode
    if let Some(prompt) = args.prompt {
        let context_length = core::models::resolve_context_length(&config.model_id);
        let result = core::llm::chat(core::llm::ChatRequest {
            config: &config,
            model: &config.model_id,
            prompt: &prompt,
            mode: "Build",
            context_length,
            confirm_destructive: Some(core::confirm::default_confirm()),
            previous_messages: None,
            options: core::llm::ChatOptions::default(),
        })
        .await?;

        // Print AI response for single prompt
        if let core::llm::ChatResult::Complete { content, .. } = result {
            println!("{}", content);
        }
        return Ok(());
    }

    // Default behavior: open the TUI (interactive chat)
    // Spawns a blocking thread to avoid runtime contention
    let config = std::sync::Arc::new(config);
    let config_clone = config.clone();
    let join_result: Result<std::io::Result<()>, tokio::task::JoinError> =
        tokio::task::spawn_blocking(move || tui::run(config_clone)).await;

    // Handle potential TUI thread failures
    join_result.map_err(|_| {
        Box::new(std::io::Error::other("TUI thread panicked")) as Box<dyn std::error::Error>
    })??;

    Ok(())
}
