//! Slash commands: prompt shortcuts with mode selection.

use crate::core::templates::{CustomTemplate, TemplatesError};
use crate::core::util::filter_by_query;

/// Lowercase names of all built-in commands (for collision check in templates).
pub const BUILTIN_NAMES: &[&str] = &[
    "init",
    "test",
    "review",
    "explain",
    "fix",
    "refactor",
    "doc",
    "commit",
    "debug",
    "why",
    "create-command",
    "update-command",
    "delete-command",
];

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
    #[allow(dead_code)]
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
        prompt_prefix: "Review Git changes in the current workspace. When Git context (branch, status) is present in your system prompt, use Bash to run `git diff` and `git diff --staged` to get the code changes. If no Git context is present (e.g. not a repo), run `git status` and `git diff` instead—or inform the user that a Git repo is required. If a scope is specified (commit hash, branch name, or PR), run `git diff <scope>`. Point out bugs, style issues, and improvements. Do not modify files—analysis only.",
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
        prompt_prefix: "Write a conventional commit message: type(scope): description. When Git context (branch, status) is present in your system prompt, run `git diff` and `git diff --staged` for the actual changes. If no Git context is present, run `git status` and `git diff` instead—or inform the user that a Git repo is required.",
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
    SlashCommand {
        name: "create-command",
        description: "Create a new custom command",
        prompt_prefix: "",
        mode: "Ask",
    },
    SlashCommand {
        name: "update-command",
        description: "Update an existing custom command",
        prompt_prefix: "",
        mode: "Ask",
    },
    SlashCommand {
        name: "delete-command",
        description: "Delete one or more custom commands",
        prompt_prefix: "",
        mode: "Ask",
    },
];

/// A resolved command (built-in or custom) used for autocomplete and execution.
#[derive(Clone, Debug)]
pub struct ResolvedCommand {
    pub name: String,
    pub description: String,
    pub prompt_prefix: String,
    pub mode: String,
    pub is_custom: bool,
}

impl ResolvedCommand {
    pub fn full_name(&self) -> String {
        format!("/{}", self.name)
    }
}

/// Merge built-in and custom commands. Built-in first (sorted), then custom (sorted).
pub fn resolve_commands(
    custom: Vec<CustomTemplate>,
) -> Result<Vec<ResolvedCommand>, TemplatesError> {
    let mut builtin: Vec<ResolvedCommand> = SLASH_COMMANDS
        .iter()
        .map(|c| ResolvedCommand {
            name: c.name.to_string(),
            description: c.description.to_string(),
            prompt_prefix: c.prompt_prefix.to_string(),
            mode: c.mode.to_string(),
            is_custom: false,
        })
        .collect();
    builtin.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let mut custom_resolved: Vec<ResolvedCommand> = custom
        .into_iter()
        .map(|t| ResolvedCommand {
            name: t.name,
            description: t.description,
            prompt_prefix: t.prompt_prefix,
            mode: t.mode,
            is_custom: true,
        })
        .collect();
    custom_resolved.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    builtin.append(&mut custom_resolved);
    Ok(builtin)
}

/// Filter resolved commands by query (case-insensitive match on name or description).
pub fn filter_commands_resolved<'a>(
    commands: &'a [ResolvedCommand],
    query: &str,
) -> Vec<&'a ResolvedCommand> {
    filter_by_query(commands, query, |c| {
        (c.name.as_str(), c.description.as_str())
    })
}

/// Filter commands by the query (everything after "/" in user input).
/// Returns commands whose name or description match (case-insensitive).
#[allow(dead_code)]
pub fn filter_commands(query: &str) -> Vec<&'static SlashCommand> {
    filter_by_query(SLASH_COMMANDS, query, |c| (c.name, c.description))
}

#[cfg(test)]
mod tests {
    use crate::core::templates::CustomTemplate;

    use super::{SLASH_COMMANDS, filter_commands, filter_commands_resolved, resolve_commands};

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

    #[test]
    fn resolve_commands_merges_builtin_and_custom() {
        let custom = vec![CustomTemplate {
            name: "security".to_string(),
            description: "Audit".to_string(),
            prompt_prefix: "Check".to_string(),
            mode: "Build".to_string(),
        }];
        let resolved = resolve_commands(custom).unwrap();
        assert!(resolved.len() > SLASH_COMMANDS.len());
        let security = resolved.iter().find(|c| c.name == "security").unwrap();
        assert!(security.is_custom);
    }

    #[test]
    fn filter_commands_resolved_matches() {
        let commands = resolve_commands(vec![]).unwrap();
        let out = filter_commands_resolved(&commands, "test");
        assert!(!out.is_empty());
        assert!(out.iter().any(|c| c.name == "test"));
    }

    #[test]
    fn resolve_commands_empty_custom_returns_builtins_only() {
        let resolved = resolve_commands(vec![]).unwrap();
        assert_eq!(resolved.len(), SLASH_COMMANDS.len());
        assert!(resolved.iter().all(|c| !c.is_custom));
    }

    #[test]
    fn resolve_commands_builtin_first_then_custom() {
        let custom = vec![
            CustomTemplate {
                name: "alpha".to_string(),
                description: "A".to_string(),
                prompt_prefix: "x".to_string(),
                mode: "Ask".to_string(),
            },
            CustomTemplate {
                name: "omega".to_string(),
                description: "Z".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Build".to_string(),
            },
        ];
        let resolved = resolve_commands(custom).unwrap();
        let first_custom_idx = resolved.iter().position(|c| c.is_custom).unwrap();
        assert_eq!(resolved[first_custom_idx].name, "alpha");
        assert_eq!(resolved[first_custom_idx + 1].name, "omega");
    }

    #[test]
    fn resolve_commands_custom_sorted_case_insensitive() {
        let custom = vec![
            CustomTemplate {
                name: "Zebra".to_string(),
                description: "Z".to_string(),
                prompt_prefix: "x".to_string(),
                mode: "Ask".to_string(),
            },
            CustomTemplate {
                name: "alpha".to_string(),
                description: "A".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Build".to_string(),
            },
        ];
        let resolved = resolve_commands(custom).unwrap();
        let custom_only: Vec<_> = resolved.iter().filter(|c| c.is_custom).collect();
        assert!(custom_only[0].name.to_lowercase() < custom_only[1].name.to_lowercase());
    }

    #[test]
    fn filter_commands_resolved_empty_query_returns_all() {
        let custom = vec![CustomTemplate {
            name: "secret".to_string(),
            description: "Hidden".to_string(),
            prompt_prefix: "x".to_string(),
            mode: "Build".to_string(),
        }];
        let commands = resolve_commands(custom).unwrap();
        let out = filter_commands_resolved(&commands, "");
        assert_eq!(out.len(), commands.len());
    }

    #[test]
    fn filter_commands_resolved_matches_custom_by_name() {
        let custom = vec![CustomTemplate {
            name: "secret".to_string(),
            description: "Hidden".to_string(),
            prompt_prefix: "x".to_string(),
            mode: "Build".to_string(),
        }];
        let commands = resolve_commands(custom).unwrap();
        let out = filter_commands_resolved(&commands, "secret");
        assert!(!out.is_empty());
        assert!(out.iter().any(|c| c.name == "secret" && c.is_custom));
    }

    #[test]
    fn filter_commands_resolved_matches_custom_by_description() {
        let custom = vec![CustomTemplate {
            name: "secret".to_string(),
            description: "Hidden audit".to_string(),
            prompt_prefix: "x".to_string(),
            mode: "Build".to_string(),
        }];
        let commands = resolve_commands(custom).unwrap();
        let out = filter_commands_resolved(&commands, "audit");
        assert!(out.iter().any(|c| c.name == "secret"));
    }
}
