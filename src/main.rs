mod confirm;
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

    if let Some(prompt) = args.prompt {
        let result = core::llm::chat(
            &prompt,
            "Build",
            Some(confirm::default_confirm()),
            None,
        )
        .await?;
        if let core::llm::ChatResult::Complete { content, .. } = result {
            println!("{}", content);
        }
        return Ok(());
    }

    // Default behavior: open the TUI (interactive chat).
    // The TUI is blocking and requires a Tokio runtime for async calls.
    // We run it in spawn_blocking with the current runtime handle.
    let handle = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || {
        // The main runtime handle can be used from spawn_blocking
        // since spawn_blocking runs in the Tokio runtime context.
        tui::run(handle)
    })
    .await
    .map_err(|_| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "TUI thread panicked",
        )) as Box<dyn std::error::Error>
    })??;

    Ok(())
}
