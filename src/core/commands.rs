//! Slash commands: prompt shortcuts with mode selection.

use crate::core::util::filter_by_query;

/// A slash command: /name triggers a prompt prefix and a mode (Ask or Build).
#[derive(Clone, Debug)]
pub struct SlashCommand {
    /// Command name without leading slash, e.g. "test".
    pub name: &'static str,
    /// Short description shown in the autocomplete list.
    pub description: &'static str,
    /// Prompt template prepended when the command is selected.
    pub prompt_prefix: &'static str,
    /// Mode passed to the LLM: "Ask" (read-only) or "Build" (full tools).
    pub mode: &'static str,
}

impl SlashCommand {
    /// Full command string including slash, e.g. "/test".
    pub fn full_name(&self) -> String {
        format!("/{}", self.name)
    }
}

/// All available slash commands.
pub static SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "test",
        description: "Write unit tests",
        prompt_prefix: "Write comprehensive unit tests. If no target specified, explore the CWD with ListDir/Read/Grep to find relevant code. Cover edge cases and typical failures. Target: ",
        mode: "Build",
    },
    SlashCommand {
        name: "review",
        description: "Review code in current project (CWD)",
        prompt_prefix: "Review the code in the current project (CWD). Explore with ListDir, Read, Grep, Glob. Point out bugs, style issues, and improvements. Focus area (optional): ",
        mode: "Ask",
    },
    SlashCommand {
        name: "explain",
        description: "Explain code or concepts simply (ELI5 style)",
        prompt_prefix: "Explain in simple terms, avoiding jargon. Break down complex parts step by step. Target: ",
        mode: "Ask",
    },
    SlashCommand {
        name: "fix",
        description: "Fix bugs",
        prompt_prefix: "Identify and fix bugs. If no code given, explore the CWD with Read/Grep. Apply fixes with Edit or Write. Target: ",
        mode: "Build",
    },
    SlashCommand {
        name: "refactor",
        description: "Refactor code",
        prompt_prefix: "Refactor for better readability and maintainability. Explore CWD if needed. Keep behavior unchanged. Target: ",
        mode: "Build",
    },
    SlashCommand {
        name: "doc",
        description: "Add documentation",
        prompt_prefix: "Add clear documentation (comments, docstrings). If no target given, explore CWD and document key modules. Target: ",
        mode: "Build",
    },
    SlashCommand {
        name: "commit",
        description: "Write commit message",
        prompt_prefix: "Write a conventional commit message: type(scope): description. Use Bash to run `git status` and `git diff` if changes not specified. Context (optional): ",
        mode: "Ask",
    },
    SlashCommand {
        name: "debug",
        description: "Debug and fix issues",
        prompt_prefix: "Debug and fix. Explore CWD with Read/Grep if needed. Identify root cause, then apply fix with Edit/Write. Issue: ",
        mode: "Build",
    },
    SlashCommand {
        name: "why",
        description: "Explain design and rationale",
        prompt_prefix: "Explain why this is written this way: design choices, trade-offs, rationale. Use Read/Grep to explore context if needed. Target: ",
        mode: "Ask",
    },
];

/// Filter commands by the query (everything after "/" in user input).
/// Returns commands whose name or description match (case-insensitive).
pub fn filter_commands(query: &str) -> Vec<&'static SlashCommand> {
    filter_by_query(SLASH_COMMANDS, query, |c| (c.name, c.description))
}
