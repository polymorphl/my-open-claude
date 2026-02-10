mod bash;
mod read;
mod write;

use serde_json::{Value, json};

pub use bash::{is_destructive, BashTool};
pub use read::ReadTool;
pub use write::WriteTool;

/// Helper to extract a string argument from tool args JSON.
pub fn str_arg(args: &Value, key: &str) -> String {
    args.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

/// Helper to build the standard tool definition structure for the API.
pub fn tool_definition(name: &str, description: &str, parameters: Value) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": parameters
        }
    })
}

/// Trait for LLM tools. Each tool provides its API definition and executes with typed arguments.
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn definition(&self) -> Value;
    fn args_preview(&self, args: &Value) -> String;
    fn execute(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>>;
}

/// All registered tools.
pub fn all() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(BashTool),
        Box::new(ReadTool),
        Box::new(WriteTool),
    ]
}

/// Tool definitions for the API (order must match `all()`).
pub fn definitions() -> Vec<Value> {
    all().iter().map(|t| t.definition()).collect()
}
