use serde::Deserialize;
use serde_json::{json, Value};

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
            "Write content to a file",
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

    fn args_preview(&self, args: &Value) -> String {
        str_arg(args, "file_path")
    }

    fn execute(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let parsed: WriteArgs = serde_json::from_value(args.clone())
            .map_err(|e| format!("Invalid arguments: {}", e))?;
        std::fs::write(&parsed.file_path, &parsed.content)?;
        Ok("OK".to_string())
    }
}
