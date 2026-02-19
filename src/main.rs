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

use clap::{Parser, Subcommand};
use dotenv::dotenv;

/// Command-line arguments for the application
///
/// Supports:
/// - Subcommands: `install`, `update`
/// - Single prompt mode (with `-p`)
/// - Interactive TUI mode (default)
#[derive(Parser)]
#[command(
    author,
    version,
    about = "An AI Assistant CLI powered by open-source models"
)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Send a single prompt then exit (without opening the TUI)
    #[arg(
        short = 'p',
        long,
        help = "Provide a prompt to get an immediate AI response"
    )]
    prompt: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install the binary to ~/.cargo/bin (run from project directory)
    Install,
    /// Update to the latest release from GitHub
    Update {
        /// Only check if an update is available, don't download
        #[arg(long)]
        check: bool,
    },
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

    // Parse command-line arguments (before logger init to choose log target)
    let args = Args::parse();

    // Handle install/update subcommands early (no config or logger needed)
    if let Some(cmd) = args.command {
        match cmd {
            Commands::Install => {
                core::install::run_install()?;
                return Ok(());
            }
            Commands::Update { check } => {
                if check {
                    core::update::run_update_check()?;
                } else {
                    core::update::run_update()?;
                }
                return Ok(());
            }
        }
    }

    // Initialize logging. In TUI mode, write to file to avoid corrupting the display.
    let mut logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"));
    if args.prompt.is_none() {
        // TUI mode: logs to file; stderr would corrupt the alternate screen
        let log_path = core::paths::cache_dir().map(|d| d.join(format!("{}.log", core::app::NAME)));
        if let Some(path) = log_path
            && let Ok(file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
        {
            logger.target(env_logger::Target::Pipe(Box::new(file)));
        }
    }
    logger.try_init().ok();

    // Load application configuration (print user-friendly message; exit uses Display not Debug)
    let config = core::config::load().unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });

    // Detect workspace (current directory, project type, AGENT.md)
    let workspace = core::workspace::detect();

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
            workspace: &workspace,
            tools_list: core::tools::all(),
            tools_defs: core::tools::definitions(),
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
        tokio::task::spawn_blocking(move || tui::run(config_clone, workspace)).await;

    // Handle potential TUI thread failures; surface the actual panic message for debugging
    match join_result {
        Ok(io_result) => io_result?,
        Err(join_err) => {
            if let Ok(panic) = join_err.try_into_panic() {
                let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic.downcast_ref::<String>() {
                    s.clone()
                } else {
                    format!("{:?}", panic)
                };
                eprintln!("TUI panic: {}", msg);
            }
            return Err(Box::new(std::io::Error::other("TUI thread panicked"))
                as Box<dyn std::error::Error>);
        }
    }

    Ok(())
}
