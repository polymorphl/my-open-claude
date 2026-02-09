use serde_json::{json, Value};
use std::fs;

/// Tool name as sent to the API and used for dispatch.
pub const NAME: &str = "Read";

pub fn definition() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": NAME,
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

pub fn execute(file_path: &str) -> String {
    fs::read_to_string(file_path)
        .unwrap_or_else(|e| format!("Error reading file: {}", e))
}
