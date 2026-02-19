//! Custom prompt templates: load, validate, and save user-defined slash commands.

mod validation;

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

impl TemplatesError {
    /// User-friendly message when falling back to built-in commands only (safe mode).
    pub fn safe_mode_message(&self) -> String {
        let detail = match self {
            TemplatesError::Io(_) => "could not read file".to_string(),
            TemplatesError::Json(_) => "invalid JSON".to_string(),
            TemplatesError::Validation(msg) => format!("validation error: {}", msg),
        };
        format!("templates.json: {} — using built-in commands only", detail)
    }
}

/// Load custom templates from `~/.config/my-open-claude/templates.json`.
/// Returns empty vec if file is absent. Errors on invalid content.
/// `builtin_names` provides built-in command names for collision check (case-insensitive).
pub fn load_templates(
    builtin_names: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<Vec<CustomTemplate>, TemplatesError> {
    let path = match paths::config_dir() {
        Some(dir) => dir.join("templates.json"),
        None => return Ok(vec![]),
    };

    if !path.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&path)?;
    let file: validation::TemplatesFile = serde_json::from_str(&content)?;
    let builtin_set: std::collections::HashSet<String> = builtin_names
        .into_iter()
        .map(|s| s.as_ref().to_lowercase())
        .collect();
    validation::validate_and_convert(file, &builtin_set)
}

/// Save custom templates to `~/.config/my-open-claude/templates.json`.
/// Creates config dir if needed.
pub fn save_templates(templates: &[CustomTemplate]) -> Result<(), TemplatesError> {
    let dir = paths::config_dir()
        .ok_or_else(|| TemplatesError::Validation("No config directory available".to_string()))?;
    fs::create_dir_all(&dir)?;

    let file = validation::TemplatesFile {
        templates: templates
            .iter()
            .map(|t| validation::TemplateEntry {
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

/// Expand {cwd} placeholder in a prompt prefix.
pub fn expand_cwd(prefix: &str, cwd: &Path) -> String {
    let cwd_str = cwd.display().to_string();
    prefix.replace("{cwd}", &cwd_str)
}

#[cfg(test)]
mod tests;
