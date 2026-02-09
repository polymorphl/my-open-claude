use serde_json::{Value, json};
use std::fs;

pub fn definition() -> Value {
    json!({
        "type": "function",
        "function": {
            "name": "Read",
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

/// Output to print: file contents (exact) vs text (with newline).
pub enum ResponseOutput {
    FileContents(String),
    Text(String),
}

/// Process the LLM response: execute tool calls or return text content.
pub fn execute_tool_call(response: &Value) -> Option<ResponseOutput> {
    let message = &response["choices"][0]["message"];

    // Check for tool_calls first
    if let Some(tool_calls) = message["tool_calls"].as_array() {
        if let Some(first_tool_call) = tool_calls.first() {
            let function = &first_tool_call["function"];
            let name = function["name"].as_str().unwrap_or("");
            let args_str = function["arguments"].as_str().unwrap_or("{}");

            if name == "Read" {
                let args: Value = serde_json::from_str(args_str).unwrap_or(json!({}));
                if let Some(file_path) = args["file_path"].as_str() {
                    return Some(ResponseOutput::FileContents(execute(file_path)));
                }
            }
        }
    }

    // Fall back to text content
    message["content"]
        .as_str()
        .map(|s| ResponseOutput::Text(s.to_string()))
}
