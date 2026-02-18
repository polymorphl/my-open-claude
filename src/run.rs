//! Application run modes: logger init, single prompt, TUI launch.

use std::io::{self, Write};
use std::sync::Arc;

use crate::cli::Args;
use crate::core;
use crate::core::config::Config;
use crate::core::workspace::Workspace;

/// Initialize env_logger. In TUI mode, writes to file to avoid corrupting the display.
pub fn init_logger(args: &Args) {
    let log_level = args.log_level();
    let mut logger =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level));

    if args.prompt.is_none() {
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
    let _ = logger.try_init();
}

/// Run single prompt mode: chat with model, print response to stdout.
pub async fn run_single_prompt(
    args: &Args,
    config: &Config,
    workspace: &Workspace,
) -> Result<(), Box<dyn std::error::Error>> {
    let prompt_arg = args.prompt.as_ref().expect("prompt is some");
    let prompt = if prompt_arg == "-" {
        std::io::read_to_string(std::io::stdin())?
    } else {
        prompt_arg.clone()
    };
    let prompt = prompt.trim();
    if prompt.is_empty() {
        eprintln!("Error: empty prompt");
        std::process::exit(1);
    }

    let model = args.model.as_deref().unwrap_or(&config.model_id);
    let mode = if args.ask { "Ask" } else { "Build" };
    let context_length = core::models::resolve_context_length(model);

    let options = if args.no_stream {
        core::llm::ChatOptions::default()
    } else {
        core::llm::ChatOptions {
            on_progress: Some(Box::new(|s| {
                let _ = writeln!(io::stderr(), "{}", s);
                let _ = io::stderr().flush();
            })),
            on_content_chunk: Some(Box::new(|s| {
                let _ = io::stdout().write_all(s.as_bytes());
                let _ = io::stdout().flush();
            })),
            ..Default::default()
        }
    };

    let result = core::llm::chat(core::llm::ChatRequest {
        config,
        model,
        prompt,
        mode,
        context_length,
        confirm_destructive: Some(core::confirm::default_confirm()),
        previous_messages: None,
        options,
        workspace,
    })
    .await?;

    if let core::llm::ChatResult::Complete { content, .. } = result {
        // In streaming mode, content was already printed via on_content_chunk
        if args.no_stream {
            println!("{}", content);
        }
    }
    Ok(())
}

/// Launch the TUI in a blocking thread. Returns on panic or IO error.
pub async fn launch_tui(
    config: Config,
    workspace: Workspace,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(config);
    let config_clone = config.clone();
    let join_result: Result<io::Result<()>, tokio::task::JoinError> =
        tokio::task::spawn_blocking(move || crate::tui::run(config_clone, workspace)).await;

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
            return Err(
                Box::new(io::Error::other("TUI thread panicked")) as Box<dyn std::error::Error>
            );
        }
    }
    Ok(())
}
