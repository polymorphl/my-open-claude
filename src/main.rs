mod core;
mod tui;

use clap::Parser;
use dotenv::dotenv;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Send a single prompt then exit (without opening the TUI).
    #[arg(short = 'p', long)]
    prompt: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let args = Args::parse();

    let config = match core::config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    if let Some(prompt) = args.prompt {
        let result = core::llm::chat(
            &config,
            &config.model_id,
            &prompt,
            "Build",
            Some(core::confirm::default_confirm()),
            None,
            None, // No progress callback in CLI mode
            None, // No content chunk callback in CLI mode
        )
        .await?;
        if let core::llm::ChatResult::Complete { content, .. } = result {
            println!("{}", content);
        }
        return Ok(());
    }

    // Default behavior: open the TUI (interactive chat).
    // The TUI runs in a blocking thread with its own Tokio runtime for chat calls,
    // avoiding block_on on the main runtime's worker threads.
    let config = std::sync::Arc::new(config);
    let config_clone = config.clone();
    let join_result: Result<std::io::Result<()>, tokio::task::JoinError> =
        tokio::task::spawn_blocking(move || tui::run(config_clone))
            .await;
    join_result
        .map_err(|_| {
            Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "TUI thread panicked",
            )) as Box<dyn std::error::Error>
        })??;

    Ok(())
}
