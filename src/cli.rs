//! CLI definitions: argument parsing, subcommands, and help text.

use clap::{ArgAction, Parser, Subcommand};
use clap_complete::Shell;

pub use clap_complete::generate;

const AFTER_HELP: &str = "\
EXAMPLES:
  my-open-claude                    Launch interactive TUI
  my-open-claude -p \"explain X\"     Single prompt, stream response to stdout
  my-open-claude -p -               Read prompt from stdin
  my-open-claude install            Install to ~/.cargo/bin
  my-open-claude update --check     Check for updates without downloading
  my-open-claude config             Show config paths and status
  my-open-claude models             List available models
  my-open-claude history list       List conversations
  my-open-claude completions bash   Generate bash completions
";

/// Command-line arguments for the application.
#[derive(Parser)]
#[command(
    author,
    version,
    about = "An AI Assistant CLI powered by open-source models",
    after_help = AFTER_HELP
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Send a single prompt then exit (without opening the TUI)
    #[arg(
        short = 'p',
        long,
        help = "Provide a prompt to get an immediate AI response (use '-' to read from stdin)"
    )]
    pub prompt: Option<String>,

    /// Override model for single prompt mode
    #[arg(short = 'm', long, help = "Model ID (e.g. anthropic/claude-haiku-4.5)")]
    pub model: Option<String>,

    /// Use read-only tools (Ask mode) for single prompt
    #[arg(long, help = "Restrict to read-only tools in prompt mode")]
    pub ask: bool,

    /// Disable streaming in prompt mode (wait for full response before printing)
    #[arg(
        long,
        help = "In prompt mode, wait for the full response instead of streaming"
    )]
    pub no_stream: bool,

    /// Increase log verbosity (use multiple times for debug)
    #[arg(short = 'v', long = "verbose", action = ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Reduce log output (errors only)
    #[arg(short = 'q', long = "quiet", global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install the binary to ~/.cargo/bin (run from project directory)
    Install,
    /// Update to the latest release from GitHub
    Update {
        /// Only check if an update is available, don't download
        #[arg(long)]
        check: bool,
    },
    /// Show config paths, model, and API key status
    Config,
    /// List available models (tool-capable)
    Models {
        /// Filter models by id or name
        #[arg(long)]
        query: Option<String>,
    },
    /// Manage conversation history
    History {
        #[command(subcommand)]
        subcommand: HistorySubcommand,
    },
    /// Generate shell completion script
    Completions {
        /// Shell to generate completions for (bash, zsh, fish, powershell, elvish)
        #[arg(value_parser = clap::value_parser!(Shell))]
        shell: Shell,
    },
}

#[derive(Subcommand)]
pub enum HistorySubcommand {
    /// List conversations
    List {
        /// Maximum number of conversations to show
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

impl Args {
    /// Log level based on -v/-q flags: error, warn, info, or debug.
    pub fn log_level(&self) -> &'static str {
        if self.quiet {
            "error"
        } else if self.verbose >= 2 {
            "debug"
        } else if self.verbose >= 1 {
            "info"
        } else {
            "warn"
        }
    }
}
