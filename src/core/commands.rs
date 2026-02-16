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
        name: "init",
        description: "Create or update AGENTS.md for this project",
        prompt_prefix: "Analyze this codebase and create or update AGENTS.md containing: \
(1) Build/lint/test commands—especially for running a single test. \
(2) Code style guidelines: imports, formatting, types, naming conventions, error handling. \
The file will be given to agentic coding agents (such as yourself) that operate in this repository. Make it about 150 lines long. \
If there are Cursor rules (Glob \".cursor/rules/*\", \".cursorrules\") or Copilot rules (Glob \".github/copilot-instructions.md\"), include them. Use Read on each path returned by Glob. \
If AGENTS.md exists: Read it first, then use Edit for each change (preserve unchanged content). If it does not exist: use Write to create it. \
Respond with a brief summary.",
        mode: "Build",
    },
    SlashCommand {
        name: "test",
        description: "Write unit tests",
        prompt_prefix: "Write comprehensive unit tests. If no target specified, explore the CWD with ListDir/Read/Grep to find relevant code. Cover edge cases and typical failures.",
        mode: "Build",
    },
    SlashCommand {
        name: "review",
        description: "Review Git changes (commit|branch|pr, defaults to uncommitted)",
        prompt_prefix: "Review Git changes in the current workspace. By default review uncommitted changes: use Bash to run `git status` and `git diff` to get the changes. If a scope is specified (commit hash, branch name, or PR), review those changes instead. Point out bugs, style issues, and improvements in the changed code. Do not modify files—analysis only.",
        mode: "Build",
    },
    SlashCommand {
        name: "explain",
        description: "Explain code or concepts simply (ELI5 style)",
        prompt_prefix: "Explain in simple terms, avoiding jargon. Break down complex parts step by step.",
        mode: "Ask",
    },
    SlashCommand {
        name: "fix",
        description: "Fix bugs",
        prompt_prefix: "Identify and fix bugs. If no code given, explore the CWD with Read/Grep. Apply fixes with Edit or Write.",
        mode: "Build",
    },
    SlashCommand {
        name: "refactor",
        description: "Refactor code",
        prompt_prefix: "Refactor for better readability and maintainability. Explore CWD if needed. Keep behavior unchanged.",
        mode: "Build",
    },
    SlashCommand {
        name: "doc",
        description: "Add documentation",
        prompt_prefix: "Add clear documentation (comments, docstrings). If no target given, explore CWD and document key modules.",
        mode: "Build",
    },
    SlashCommand {
        name: "commit",
        description: "Write commit message",
        prompt_prefix: "Write a conventional commit message: type(scope): description. Use Bash to run `git status` and `git diff` if changes not specified.",
        mode: "Ask",
    },
    SlashCommand {
        name: "debug",
        description: "Debug and fix issues",
        prompt_prefix: "Debug and fix. Explore CWD with Read/Grep if needed. Identify root cause, then apply fix with Edit/Write.",
        mode: "Build",
    },
    SlashCommand {
        name: "why",
        description: "Explain design and rationale",
        prompt_prefix: "Explain why this is written this way: design choices, trade-offs, rationale. Use Read/Grep to explore context if needed.",
        mode: "Ask",
    },
];

/// Filter commands by the query (everything after "/" in user input).
/// Returns commands whose name or description match (case-insensitive).
pub fn filter_commands(query: &str) -> Vec<&'static SlashCommand> {
    filter_by_query(SLASH_COMMANDS, query, |c| (c.name, c.description))
}

#[cfg(test)]
mod tests {
    use super::{SLASH_COMMANDS, filter_commands};

    #[test]
    fn filter_empty_returns_all() {
        let out = filter_commands("");
        assert_eq!(out.len(), SLASH_COMMANDS.len());
    }

    #[test]
    fn filter_by_name() {
        let out = filter_commands("test");
        assert!(!out.is_empty());
        assert!(out.iter().any(|c| c.name == "test"));
    }

    #[test]
    fn filter_by_partial_name() {
        let out = filter_commands("rev");
        assert!(out.iter().any(|c| c.name == "review"));
    }

    #[test]
    fn filter_case_insensitive() {
        let out = filter_commands("TEST");
        assert!(out.iter().any(|c| c.name == "test"));
    }

    #[test]
    fn filter_by_description() {
        let out = filter_commands("unit tests");
        assert!(out.iter().any(|c| c.name == "test"));
    }

    #[test]
    fn filter_no_match() {
        let out = filter_commands("xyznonexistent");
        assert!(out.is_empty());
    }
}
