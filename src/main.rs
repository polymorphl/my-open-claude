//! # My Open Claude - AI Assistant CLI
//!
//! Main entry point. Handles CLI parsing, subcommand dispatch, and launching
//! either single-prompt mode or the interactive TUI.

mod cli;
mod core;
mod run;
mod tui;

use std::env;

use clap::{CommandFactory, Parser};
use cli::{Args, Commands, ConfigSubcommand, HistorySubcommand};
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load stored API key if not in env (before dotenv, so cwd .env can override)
    if env::var("OPENROUTER_API_KEY")
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
        && let Some(key) = core::api_key::load_api_key()
    {
        // SAFETY: single-threaded early startup, no other env access concurrent
        unsafe { env::set_var("OPENROUTER_API_KEY", key) };
    }
    dotenv().ok();
    let args = Args::parse();

    // Early subcommands (no config needed)
    if let Some(cmd) = args.command.as_ref()
        && dispatch_early_command(cmd)?.is_some()
    {
        return Ok(());
    }

    run::init_logger(&args);
    let config = load_config_or_exit();

    // Models subcommand (needs config)
    if let Some(Commands::Models { query }) = &args.command {
        core::cli::run_models(&config, query.as_deref()).await;
        return Ok(());
    }

    let workspace = core::workspace::detect();

    if args.prompt.is_some() {
        run::run_single_prompt(&args, &config, &workspace).await?;
        return Ok(());
    }

    run::launch_tui(config, workspace).await
}

/// Dispatch install, update, config, completions, history. Returns Some(()) if handled.
fn dispatch_early_command(cmd: &Commands) -> Result<Option<()>, Box<dyn std::error::Error>> {
    match cmd {
        Commands::Install => {
            core::install::run_install()?;
            Ok(Some(()))
        }
        Commands::Update { check } => {
            if *check {
                core::update::run_update_check()?;
            } else {
                core::update::run_update()?;
            }
            Ok(Some(()))
        }
        Commands::Config { subcommand } => {
            match subcommand {
                ConfigSubcommand::Show => {
                    core::cli::run_config();
                }
                ConfigSubcommand::SetApiKey { api_key } => {
                    core::cli::run_config_set_api_key(api_key.clone());
                }
            }
            Ok(Some(()))
        }
        Commands::Completions { shell } => {
            let mut app = Args::command();
            cli::generate(*shell, &mut app, core::app::NAME, &mut std::io::stdout());
            Ok(Some(()))
        }
        Commands::History { subcommand } => {
            let HistorySubcommand::List { limit } = subcommand;
            core::cli::run_history_list(*limit);
            Ok(Some(()))
        }
        Commands::Models { .. } => Ok(None),
    }
}

fn load_config_or_exit() -> core::config::Config {
    core::config::load().unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    })
}
