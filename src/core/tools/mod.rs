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

pub use bash::BashTool;
pub use edit::EditTool;
pub use glob_tool::GlobTool;
pub use grep::GrepTool;
pub use list_dir::ListDirTool;
pub use read::ReadTool;
pub use write::WriteTool;

/// Default path for search tools (current directory).
pub const DEFAULT_SEARCH_PATH: &str = ".";

/// Returns the default search path for tools (typically the current directory ".").
pub fn default_search_path() -> String {
    DEFAULT_SEARCH_PATH.to_string()
}

/// Default max results for Grep (matches).
pub const GREP_DEFAULT_MAX_RESULTS: usize = 50;

/// Default max results for Glob (files).
pub const GLOB_DEFAULT_MAX_RESULTS: usize = 100;

/// Helper to extract a string argument from tool args JSON.
pub fn str_arg(args: &Value, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
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

/// Error type for tool execution (Send + Sync for use across async/thread boundaries).
pub type ToolError = Box<dyn std::error::Error + Send + Sync>;

/// Max output size for Read and Bash tool results (32 KB).
pub const MAX_OUTPUT_LARGE: usize = 32 * 1024;
/// Max output size for Grep, ListDir, Glob tool results (16 KB).
pub const MAX_OUTPUT_SMALL: usize = 16 * 1024;

/// Trait for LLM tools. Each tool provides its API definition and executes with typed arguments.
pub trait Tool: Send + Sync {
    /// Unique tool name (e.g. "Read", "Bash") used in API calls.
    fn name(&self) -> &'static str;
    /// JSON schema for the API: type, function name, description, parameters.
    fn definition(&self) -> Value;
    /// Short preview of arguments for display in progress/log (e.g. path, command).
    fn args_preview(&self, args: &Value) -> String;
    /// Execute the tool with the given arguments. Returns output string or error.
    fn execute(&self, args: &Value) -> Result<String, ToolError>;

    /// Optional: max output size in bytes. Default: None (unlimited).
    fn output_limit(&self) -> Option<usize> {
        None
    }

    /// Optional: disabled in Ask mode (read-only)? Default: false.
    fn disabled_in_ask_mode(&self) -> bool {
        false
    }

    /// Optional: may require user confirmation (e.g. destructive Bash command). Default: false.
    fn may_need_confirmation(&self, args: &Value) -> bool {
        let _ = args;
        false
    }

    /// Optional: is this path an init file (AGENT.md/AGENTS.md) that should be written only once per session? Default: false.
    fn is_init_file_target(&self, file_path: &str) -> bool {
        let _ = file_path;
        false
    }
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
