use serde::Deserialize;
use serde_json::{json, Value};

use super::{str_arg, tool_definition};

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
        tool_definition(
            self.name(),
            "Read and return the contents of a file",
            json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The path to the file to read"
                    }
                },
                "required": ["file_path"]
            }),
        )
    }

    fn args_preview(&self, args: &Value) -> String {
        str_arg(args, "file_path")
    }

    fn execute(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let parsed: ReadArgs = serde_json::from_value(args.clone())
            .map_err(|e| format!("Invalid arguments: {}", e))?;
        std::fs::read_to_string(&parsed.file_path).map_err(Into::into)
    }
}
