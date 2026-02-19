use serde::Deserialize;
use serde_json::{Value, json};

use super::{str_arg, tool_definition};

#[derive(Debug, Deserialize)]
pub struct WriteArgs {
    pub file_path: String,
    pub content: String,
}

pub struct WriteTool;

impl super::Tool for WriteTool {
    fn name(&self) -> &'static str {
        "Write"
    }

    fn definition(&self) -> Value {
        tool_definition(
            self.name(),
            "Write content to a file. OVERWRITES the entire file. For existing files that need updates, prefer Edit (targeted search-and-replace) to preserve unchanged content.",
            json!({
                "type": "object",
                "required": ["file_path", "content"],
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The path of the file to write to"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file"
                    }
                }
            }),
        )
    }

    fn disabled_in_ask_mode(&self) -> bool {
        true
    }

    fn is_init_file_target(&self, file_path: &str) -> bool {
        std::path::Path::new(file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|n| {
                ["agent.md", "agents.md"]
                    .iter()
                    .any(|&init| n.eq_ignore_ascii_case(init))
            })
            .unwrap_or(false)
    }

    fn args_preview(&self, args: &Value) -> String {
        str_arg(args, "file_path")
    }

    fn execute(&self, args: &Value) -> Result<String, super::ToolError> {
        let parsed: WriteArgs = serde_json::from_value(args.clone())
            .map_err(|e| std::io::Error::other(format!("Invalid arguments: {}", e)))?;
        std::fs::write(&parsed.file_path, &parsed.content)?;
        Ok("OK".to_string())
    }
}
