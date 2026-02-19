//! Slash commands: prompt shortcuts with mode selection.
//!
//! Built-in commands are loaded from `config/builtin-commands.json` (embedded at compile time).

use std::sync::OnceLock;

use serde::Deserialize;

use crate::core::templates::{CustomTemplate, TemplatesError};
use crate::core::util::filter_by_query;

/// Built-in command definition (loaded from config).
#[derive(Clone, Debug)]
pub struct BuiltinCommand {
    pub name: String,
    pub description: String,
    pub prompt_prefix: String,
    pub mode: String,
}

impl BuiltinCommand {
    /// Full command string including slash, e.g. "/test".
    #[allow(dead_code)]
    pub fn full_name(&self) -> String {
        format!("/{}", self.name)
    }
}

#[derive(Debug, Deserialize)]
struct BuiltinCommandEntry {
    name: String,
    description: String,
    prompt_prefix: String,
    mode: String,
}

fn load_builtin_commands() -> Vec<BuiltinCommand> {
    let json = include_str!("../../config/builtin-commands.json");
    let entries: Vec<BuiltinCommandEntry> =
        serde_json::from_str(json).expect("builtin-commands.json must be valid");
    entries
        .into_iter()
        .map(|e| BuiltinCommand {
            name: e.name,
            description: e.description,
            prompt_prefix: e.prompt_prefix,
            mode: e.mode,
        })
        .collect()
}

static BUILTIN_COMMANDS: OnceLock<Vec<BuiltinCommand>> = OnceLock::new();

/// Returns all built-in slash commands, loading from config on first access.
pub fn builtin_commands() -> &'static [BuiltinCommand] {
    BUILTIN_COMMANDS.get_or_init(load_builtin_commands)
}

/// Returns true if the given name conflicts with a built-in command (case-insensitive).
pub fn is_builtin_name(name: &str) -> bool {
    builtin_commands()
        .iter()
        .any(|c| c.name.eq_ignore_ascii_case(name))
}

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
    let mut builtin: Vec<ResolvedCommand> = builtin_commands()
        .iter()
        .map(|c| ResolvedCommand {
            name: c.name.clone(),
            description: c.description.clone(),
            prompt_prefix: c.prompt_prefix.clone(),
            mode: c.mode.clone(),
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

/// Filter built-in commands by the query (everything after "/" in user input).
/// Returns commands whose name or description match (case-insensitive).
#[allow(dead_code)]
pub fn filter_commands(query: &str) -> Vec<&'static BuiltinCommand> {
    filter_by_query(builtin_commands(), query, |c| {
        (c.name.as_str(), c.description.as_str())
    })
}

#[cfg(test)]
mod tests {
    use crate::core::templates::CustomTemplate;

    use super::{builtin_commands, filter_commands, filter_commands_resolved, resolve_commands};

    #[test]
    fn filter_empty_returns_all() {
        let out = filter_commands("");
        assert_eq!(out.len(), builtin_commands().len());
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
        assert!(resolved.len() > builtin_commands().len());
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
        assert_eq!(resolved.len(), builtin_commands().len());
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
