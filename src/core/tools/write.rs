use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;

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
        json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": "Write content to a file",
                "parameters": {
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
                }
            }
        })
    }

    fn args_preview(&self, args: &Value) -> String {
        args.get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }

    fn execute(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let parsed: WriteArgs = serde_json::from_value(args.clone())
            .map_err(|e| format!("Invalid arguments: {}", e))?;
        fs::write(&parsed.file_path, &parsed.content)?;
        Ok("OK".to_string())
    }
}
