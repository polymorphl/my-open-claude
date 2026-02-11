mod bash;
mod edit;
mod glob_tool;
mod grep;
mod ignore;
mod list_dir;
mod read;
mod write;

use std::sync::OnceLock;

use serde_json::{Value, json};

pub use bash::{is_destructive, BashTool};
pub use edit::EditTool;
pub use glob_tool::GlobTool;
pub use grep::GrepTool;
pub use list_dir::ListDirTool;
pub use read::ReadTool;
pub use write::WriteTool;

/// Default path for search tools (current directory).
pub fn default_search_path() -> String {
    ".".to_string()
}

/// Default max results for Grep (matches).
pub const GREP_DEFAULT_MAX_RESULTS: usize = 50;

/// Default max results for Glob (files).
pub const GLOB_DEFAULT_MAX_RESULTS: usize = 100;

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

static CACHED_TOOLS: OnceLock<Vec<Box<dyn Tool>>> = OnceLock::new();
static CACHED_DEFINITIONS: OnceLock<Vec<Value>> = OnceLock::new();

fn init_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(BashTool),
        Box::new(ReadTool),
        Box::new(WriteTool),
        Box::new(EditTool),
        Box::new(GrepTool),
        Box::new(ListDirTool),
        Box::new(GlobTool),
    ]
}

/// All registered tools. Cached after first call.
pub fn all() -> &'static [Box<dyn Tool>] {
    CACHED_TOOLS.get_or_init(init_tools)
}

/// Tool definitions for the API (order must match `all()`). Cached after first call.
pub fn definitions() -> &'static [Value] {
    CACHED_DEFINITIONS.get_or_init(|| all().iter().map(|t| t.definition()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_arg_present() {
        let args = serde_json::json!({"path": "/tmp/foo"});
        assert_eq!(str_arg(&args, "path"), "/tmp/foo");
    }

    #[test]
    fn str_arg_missing_returns_empty() {
        let args = serde_json::json!({"other": "x"});
        assert_eq!(str_arg(&args, "path"), "");
    }

    #[test]
    fn tool_definition_structure() {
        let def = tool_definition(
            "Read",
            "Read file contents",
            serde_json::json!({"type": "object"}),
        );
        assert_eq!(def["type"], "function");
        assert_eq!(def["function"]["name"], "Read");
        assert_eq!(def["function"]["description"], "Read file contents");
        assert_eq!(def["function"]["parameters"]["type"], "object");
    }
}
