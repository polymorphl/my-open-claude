//! Read tool — read file contents, optionally a specific line range.
//!
//! Use `start_line` and `end_line` (1-based) to read only part of a large file,
//! saving tokens. Omit both for the full file.

use serde::Deserialize;
use serde_json::{Value, json};

use super::{str_arg, tool_definition};

#[derive(Debug, Deserialize)]
pub struct ReadArgs {
    pub file_path: String,
    #[serde(default)]
    pub start_line: Option<u64>,
    #[serde(default)]
    pub end_line: Option<u64>,
}

pub struct ReadTool;

impl super::Tool for ReadTool {
    fn name(&self) -> &'static str {
        "Read"
    }

    fn definition(&self) -> Value {
        tool_definition(
            self.name(),
            "Read file contents. Optionally use start_line and end_line (1-based, inclusive) to read only a range of lines — useful for large files to save tokens.",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The path to the file to read"
                    },
                    "start_line": {
                        "type": "integer",
                        "description": "First line to include (1-based). Omit for full file or to start from the beginning."
                    },
                    "end_line": {
                        "type": "integer",
                        "description": "Last line to include (1-based, inclusive). Omit to read until end of file."
                    }
                },
                "required": ["file_path"]
            }),
        )
    }

    fn args_preview(&self, args: &Value) -> String {
        let path = str_arg(args, "file_path");
        let start = args.get("start_line").and_then(|v| v.as_u64());
        let end = args.get("end_line").and_then(|v| v.as_u64());
        match (start, end) {
            (Some(s), Some(e)) => format!("{} (lines {}-{})", path, s, e),
            (Some(s), None) => format!("{} (from line {})", path, s),
            (None, Some(e)) => format!("{} (up to line {})", path, e),
            (None, None) => path,
        }
    }

    fn execute(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let parsed: ReadArgs = serde_json::from_value(args.clone())
            .map_err(|e| format!("Invalid arguments: {}", e))?;

        let content = std::fs::read_to_string(&parsed.file_path)?;
        if parsed.start_line.is_none() && parsed.end_line.is_none() {
            return Ok(content);
        }

        let lines: Vec<&str> = content.lines().collect();
        let line_count = lines.len();
        let start = parsed.start_line.unwrap_or(1).max(1) as usize;
        let end = parsed
            .end_line
            .unwrap_or(u64::MAX)
            .min(line_count as u64)
            .max(start as u64) as usize;
        if lines.is_empty() {
            return Ok(String::new());
        }
        if start > line_count {
            return Err(
                format!("start_line {} is beyond file ({} lines)", start, line_count).into(),
            );
        }
        let end = end.min(line_count);
        let selected: Vec<&str> = lines[(start - 1)..end].to_vec();
        Ok(selected.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tools::Tool;
    use serde_json::json;

    #[test]
    fn read_full_file() {
        let tool = ReadTool;
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "line1\nline2\nline3").unwrap();
        let args = json!({"file_path": file.path().to_str().unwrap()});
        let result = tool.execute(&args).unwrap();
        assert_eq!(result, "line1\nline2\nline3");
    }

    #[test]
    fn read_line_range() {
        let tool = ReadTool;
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "a\nb\nc\nd\ne").unwrap();
        let args = json!({
            "file_path": file.path().to_str().unwrap(),
            "start_line": 2,
            "end_line": 4
        });
        let result = tool.execute(&args).unwrap();
        assert_eq!(result, "b\nc\nd");
    }

    #[test]
    fn read_from_line_to_end() {
        let tool = ReadTool;
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "a\nb\nc").unwrap();
        let args = json!({
            "file_path": file.path().to_str().unwrap(),
            "start_line": 2
        });
        let result = tool.execute(&args).unwrap();
        assert_eq!(result, "b\nc");
    }

    #[test]
    fn read_start_line_beyond_file_fails() {
        let tool = ReadTool;
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "a\nb").unwrap();
        let args = json!({
            "file_path": file.path().to_str().unwrap(),
            "start_line": 10
        });
        let err = tool.execute(&args).unwrap_err();
        assert!(err.to_string().contains("beyond file"));
    }

    #[test]
    fn read_empty_file() {
        let tool = ReadTool;
        let file = tempfile::NamedTempFile::new().unwrap();
        let args = json!({"file_path": file.path().to_str().unwrap()});
        let result = tool.execute(&args).unwrap();
        assert_eq!(result, "");
    }
}
