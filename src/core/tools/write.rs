use serde_json::{json, Value};
use std::fs;

/// Tool name as sent to the API and used for dispatch.
pub const NAME: &str = "Write";

pub fn definition() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": NAME,
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

pub fn execute(file_path: &str, content: &str) -> String {
    fs::write(file_path, content)
        .map(|()| "OK".to_string())
        .unwrap_or_else(|e| format!("Error writing file: {}", e))
}
