//! Custom prompt templates: load, validate, and save user-defined slash commands.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::core::paths;

/// A user-defined template (custom slash command).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomTemplate {
    pub name: String,
    pub description: String,
    pub prompt_prefix: String,
    pub mode: String,
}

/// Error loading or saving templates.
#[derive(Debug, thiserror::Error)]
pub enum TemplatesError {
    #[error("Failed to read templates file: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Validation error: {0}")]
    Validation(String),
}

/// JSON structure on disk.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TemplatesFile {
    templates: Vec<TemplateEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TemplateEntry {
    name: String,
    description: String,
    prompt_prefix: String,
    mode: String,
}

/// Load custom templates from `~/.config/my-open-claude/templates.json`.
/// Returns empty vec if file is absent. Errors on invalid content.
/// `builtin_names` must contain lowercase built-in command names for collision check.
pub fn load_templates(builtin_names: &[&str]) -> Result<Vec<CustomTemplate>, TemplatesError> {
    let path = match paths::config_dir() {
        Some(dir) => dir.join("templates.json"),
        None => return Ok(vec![]),
    };

    if !path.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&path)?;
    let file: TemplatesFile = serde_json::from_str(&content)?;
    validate_and_convert(file, builtin_names)
}

/// Save custom templates to `~/.config/my-open-claude/templates.json`.
/// Creates config dir if needed.
pub fn save_templates(templates: &[CustomTemplate]) -> Result<(), TemplatesError> {
    let dir = paths::config_dir()
        .ok_or_else(|| TemplatesError::Validation("No config directory available".to_string()))?;
    fs::create_dir_all(&dir)?;

    let file = TemplatesFile {
        templates: templates
            .iter()
            .map(|t| TemplateEntry {
                name: t.name.clone(),
                description: t.description.clone(),
                prompt_prefix: t.prompt_prefix.clone(),
                mode: t.mode.clone(),
            })
            .collect(),
    };
    let path = dir.join("templates.json");
    let content = serde_json::to_string_pretty(&file)?;
    fs::write(path, content)?;
    Ok(())
}

pub(crate) fn validate_and_convert(
    file: TemplatesFile,
    builtin_names: &[&str],
) -> Result<Vec<CustomTemplate>, TemplatesError> {
    let mut seen = HashSet::new();
    let mut result = Vec::with_capacity(file.templates.len());

    for (i, entry) in file.templates.into_iter().enumerate() {
        // name: alphanumeric only
        if entry.name.is_empty() {
            return Err(TemplatesError::Validation(format!(
                "Template at index {}: name cannot be empty",
                i
            )));
        }
        if !entry.name.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(TemplatesError::Validation(format!(
                "Template '{}': name must be alphanumeric only",
                entry.name
            )));
        }
        let name_lower = entry.name.to_lowercase();

        // collision with built-in
        if builtin_names.contains(&name_lower.as_str()) {
            return Err(TemplatesError::Validation(format!(
                "Template '{}': name conflicts with built-in command",
                entry.name
            )));
        }

        // duplicate within file
        if !seen.insert(name_lower.clone()) {
            return Err(TemplatesError::Validation(format!(
                "Duplicate template name '{}'",
                entry.name
            )));
        }

        // mode
        if entry.mode != "Ask" && entry.mode != "Build" {
            return Err(TemplatesError::Validation(format!(
                "Template '{}': mode must be 'Ask' or 'Build', got '{}'",
                entry.name, entry.mode
            )));
        }

        // description and prompt_prefix non-empty
        if entry.description.trim().is_empty() {
            return Err(TemplatesError::Validation(format!(
                "Template '{}': description cannot be empty",
                entry.name
            )));
        }
        if entry.prompt_prefix.trim().is_empty() {
            return Err(TemplatesError::Validation(format!(
                "Template '{}': prompt_prefix cannot be empty",
                entry.name
            )));
        }

        result.push(CustomTemplate {
            name: entry.name,
            description: entry.description,
            prompt_prefix: entry.prompt_prefix,
            mode: entry.mode,
        });
    }

    Ok(result)
}

/// Expand {cwd} placeholder in a prompt prefix.
pub fn expand_cwd(prefix: &str, cwd: &Path) -> String {
    let cwd_str = cwd.display().to_string();
    prefix.replace("{cwd}", &cwd_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILTIN: &[&str] = &["test", "init", "create-command"];

    #[test]
    fn expand_cwd_replaces_placeholder() {
        let cwd = Path::new("/home/user/project");
        let out = expand_cwd("CWD: {cwd}", cwd);
        assert_eq!(out, "CWD: /home/user/project");
    }

    #[test]
    fn expand_cwd_preserves_without_placeholder() {
        let cwd = Path::new("/home");
        let out = expand_cwd("no placeholder", cwd);
        assert_eq!(out, "no placeholder");
    }

    #[test]
    fn validate_rejects_duplicate_names() {
        let file = TemplatesFile {
            templates: vec![
                TemplateEntry {
                    name: "a".to_string(),
                    description: "x".to_string(),
                    prompt_prefix: "y".to_string(),
                    mode: "Ask".to_string(),
                },
                TemplateEntry {
                    name: "a".to_string(),
                    description: "x".to_string(),
                    prompt_prefix: "y".to_string(),
                    mode: "Ask".to_string(),
                },
            ],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("Duplicate"));
    }

    #[test]
    fn validate_rejects_builtin_collision() {
        let file = TemplatesFile {
            templates: vec![TemplateEntry {
                name: "test".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            }],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("built-in"));
    }

    #[test]
    fn validate_accepts_valid_custom() {
        let file = TemplatesFile {
            templates: vec![TemplateEntry {
                name: "security".to_string(),
                description: "Audit".to_string(),
                prompt_prefix: "Check {cwd}".to_string(),
                mode: "Build".to_string(),
            }],
        };
        let out = validate_and_convert(file, BUILTIN).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].name, "security");
    }

    #[test]
    fn validate_rejects_empty_name() {
        let file = TemplatesFile {
            templates: vec![TemplateEntry {
                name: "".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            }],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("cannot be empty"));
    }

    #[test]
    fn validate_rejects_name_with_spaces() {
        let file = TemplatesFile {
            templates: vec![TemplateEntry {
                name: "my command".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            }],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("alphanumeric"));
    }

    #[test]
    fn validate_rejects_name_with_hyphens() {
        let file = TemplatesFile {
            templates: vec![TemplateEntry {
                name: "my-command".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            }],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("alphanumeric"));
    }

    #[test]
    fn validate_rejects_name_with_special_chars() {
        let file = TemplatesFile {
            templates: vec![TemplateEntry {
                name: "cmd!".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            }],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("alphanumeric"));
    }

    #[test]
    fn validate_rejects_builtin_collision_case_insensitive() {
        let file = TemplatesFile {
            templates: vec![TemplateEntry {
                name: "TEST".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            }],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("built-in"));
    }

    #[test]
    fn validate_rejects_duplicate_names_case_insensitive() {
        let file = TemplatesFile {
            templates: vec![
                TemplateEntry {
                    name: "Foo".to_string(),
                    description: "x".to_string(),
                    prompt_prefix: "y".to_string(),
                    mode: "Ask".to_string(),
                },
                TemplateEntry {
                    name: "foo".to_string(),
                    description: "x".to_string(),
                    prompt_prefix: "y".to_string(),
                    mode: "Ask".to_string(),
                },
            ],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("Duplicate"));
    }

    #[test]
    fn validate_rejects_invalid_mode() {
        let file = TemplatesFile {
            templates: vec![TemplateEntry {
                name: "custom".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Random".to_string(),
            }],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("mode"));
        assert!(err.to_string().contains("Ask") || err.to_string().contains("Build"));
    }

    #[test]
    fn validate_rejects_mode_lowercase() {
        let file = TemplatesFile {
            templates: vec![TemplateEntry {
                name: "custom".to_string(),
                description: "x".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "ask".to_string(),
            }],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("mode"));
    }

    #[test]
    fn validate_rejects_empty_description() {
        let file = TemplatesFile {
            templates: vec![TemplateEntry {
                name: "custom".to_string(),
                description: "".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            }],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("description"));
    }

    #[test]
    fn validate_rejects_whitespace_only_description() {
        let file = TemplatesFile {
            templates: vec![TemplateEntry {
                name: "custom".to_string(),
                description: "   \t  ".to_string(),
                prompt_prefix: "y".to_string(),
                mode: "Ask".to_string(),
            }],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("description"));
    }

    #[test]
    fn validate_rejects_empty_prompt_prefix() {
        let file = TemplatesFile {
            templates: vec![TemplateEntry {
                name: "custom".to_string(),
                description: "x".to_string(),
                prompt_prefix: "".to_string(),
                mode: "Ask".to_string(),
            }],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("prompt_prefix"));
    }

    #[test]
    fn validate_rejects_whitespace_only_prompt_prefix() {
        let file = TemplatesFile {
            templates: vec![TemplateEntry {
                name: "custom".to_string(),
                description: "x".to_string(),
                prompt_prefix: "\n\t  ".to_string(),
                mode: "Ask".to_string(),
            }],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("prompt_prefix"));
    }

    #[test]
    fn validate_accepts_empty_file() {
        let file = TemplatesFile {
            templates: vec![],
        };
        let out = validate_and_convert(file, BUILTIN).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn validate_accepts_multiple_valid_templates() {
        let file = TemplatesFile {
            templates: vec![
                TemplateEntry {
                    name: "alpha".to_string(),
                    description: "First".to_string(),
                    prompt_prefix: "Do A".to_string(),
                    mode: "Ask".to_string(),
                },
                TemplateEntry {
                    name: "beta".to_string(),
                    description: "Second".to_string(),
                    prompt_prefix: "Do B".to_string(),
                    mode: "Build".to_string(),
                },
            ],
        };
        let out = validate_and_convert(file, BUILTIN).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].name, "alpha");
        assert_eq!(out[1].name, "beta");
    }

    #[test]
    fn validate_fails_first_invalid_among_many() {
        let file = TemplatesFile {
            templates: vec![
                TemplateEntry {
                    name: "valid".to_string(),
                    description: "x".to_string(),
                    prompt_prefix: "y".to_string(),
                    mode: "Ask".to_string(),
                },
                TemplateEntry {
                    name: "".to_string(),
                    description: "x".to_string(),
                    prompt_prefix: "y".to_string(),
                    mode: "Ask".to_string(),
                },
                TemplateEntry {
                    name: "also invalid".to_string(),
                    description: "x".to_string(),
                    prompt_prefix: "y".to_string(),
                    mode: "Ask".to_string(),
                },
            ],
        };
        let err = validate_and_convert(file, BUILTIN).unwrap_err();
        assert!(err.to_string().contains("index 1") || err.to_string().contains("cannot be empty"));
    }
}
