//! Template validation: disk format and conversion to CustomTemplate.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::TemplatesError;

/// JSON structure on disk.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TemplatesFile {
    pub(super) templates: Vec<TemplateEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TemplateEntry {
    pub name: String,
    pub description: String,
    pub prompt_prefix: String,
    pub mode: String,
}

/// Validate file entries and convert to CustomTemplate list.
pub(crate) fn validate_and_convert(
    file: TemplatesFile,
    builtin_names: &[&str],
) -> Result<Vec<super::CustomTemplate>, TemplatesError> {
    let mut seen = HashSet::new();
    let mut result = Vec::with_capacity(file.templates.len());

    for (i, entry) in file.templates.into_iter().enumerate() {
        // name: alphanumeric, hyphens, underscores
        if entry.name.is_empty() {
            return Err(TemplatesError::Validation(format!(
                "Template at index {}: name cannot be empty",
                i
            )));
        }
        if !entry
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(TemplatesError::Validation(format!(
                "Template '{}': name must contain only letters, numbers, hyphens, and underscores",
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

        result.push(super::CustomTemplate {
            name: entry.name,
            description: entry.description,
            prompt_prefix: entry.prompt_prefix,
            mode: entry.mode,
        });
    }

    Ok(result)
}
