use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;

#[derive(Debug, Deserialize)]
pub struct ReadArgs {
    pub file_path: String,
}

pub struct ReadTool;

impl super::Tool for ReadTool {
    fn name(&self) -> &'static str {
        "Read"
    }

    fn definition(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": "Read and return the contents of a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "file_path": {
                            "type": "string",
                            "description": "The path to the file to read"
                        }
                    },
                    "required": ["file_path"]
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
        let parsed: ReadArgs = serde_json::from_value(args.clone())
            .map_err(|e| format!("Invalid arguments: {}", e))?;
        fs::read_to_string(&parsed.file_path).map_err(Into::into)
    }
}
