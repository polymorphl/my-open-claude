//! Edit tool — targeted search-and-replace in a file.
//!
//! Safer than Write: only replaces a specific occurrence, fails if the match
//! is ambiguous (0 or 2+ occurrences).

use serde::Deserialize;
use serde_json::{Value, json};
use std::fs;

use super::{str_arg, tool_definition};

#[derive(Debug, Deserialize)]
struct EditArgs {
    file_path: String,
    old_string: String,
    new_string: String,
}

pub struct EditTool;

impl super::Tool for EditTool {
    fn name(&self) -> &'static str {
        "Edit"
    }

    fn definition(&self) -> Value {
        tool_definition(
            self.name(),
            "Replace a specific string in a file. The old_string must match exactly once in the file. This is safer and more token-efficient than rewriting the whole file with Write.",
            json!({
                "type": "object",
                "required": ["file_path", "old_string", "new_string"],
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Path to the file to edit"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "Exact text to find in the file (must occur exactly once)"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "Replacement text"
                    }
                }
            }),
        )
    }

    fn args_preview(&self, args: &Value) -> String {
        str_arg(args, "file_path")
    }

    fn execute(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let parsed: EditArgs = serde_json::from_value(args.clone())
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        let content = fs::read_to_string(&parsed.file_path)
            .map_err(|e| format!("Cannot read file '{}': {}", parsed.file_path, e))?;

        // Count occurrences
        let count = content.matches(&parsed.old_string).count();
        if count == 0 {
            return Err(format!(
                "old_string not found in '{}'. Make sure it matches the file content exactly (including whitespace and indentation).",
                parsed.file_path
            )
            .into());
        }
        if count > 1 {
            return Err(format!(
                "old_string found {} times in '{}'. It must occur exactly once. Add more surrounding context to make it unique.",
                count, parsed.file_path
            )
            .into());
        }

        let new_content = content.replacen(&parsed.old_string, &parsed.new_string, 1);
        fs::write(&parsed.file_path, &new_content)
            .map_err(|e| format!("Cannot write file '{}': {}", parsed.file_path, e))?;

        Ok(format!(
            "OK — replaced {} bytes with {} bytes in {}",
            parsed.old_string.len(),
            parsed.new_string.len(),
            parsed.file_path
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tools::Tool;
    use serde_json::json;

    #[test]
    fn edit_replaces_unique_occurrence() {
        let tool = EditTool;
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "hello world\n").unwrap();
        let args = json!({
            "file_path": file.path().to_str().unwrap(),
            "old_string": "world",
            "new_string": "earth"
        });
        let result = tool.execute(&args).unwrap();
        assert!(result.contains("OK"));
        assert_eq!(std::fs::read_to_string(file.path()).unwrap(), "hello earth\n");
    }

    #[test]
    fn edit_fails_when_old_string_not_found() {
        let tool = EditTool;
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "hello world").unwrap();
        let args = json!({
            "file_path": file.path().to_str().unwrap(),
            "old_string": "xyz",
            "new_string": "replacement"
        });
        let err = tool.execute(&args).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn edit_fails_when_old_string_ambiguous() {
        let tool = EditTool;
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "foo foo foo").unwrap();
        let args = json!({
            "file_path": file.path().to_str().unwrap(),
            "old_string": "foo",
            "new_string": "bar"
        });
        let err = tool.execute(&args).unwrap_err();
        assert!(err.to_string().contains("found 3 times"));
    }
}
