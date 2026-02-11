//! Execute a single tool call from the agent loop.

use serde_json::{Value, json};

use crate::core::confirm::ConfirmDestructive;
use crate::core::tools;

use super::ChatError;
use super::ChatResult;
use super::ConfirmState;

const WRITE_NAME: &str = "Write";
const EDIT_NAME: &str = "Edit";
const BASH_NAME: &str = "Bash";
const READ_NAME: &str = "Read";
const GREP_NAME: &str = "Grep";
const LIST_DIR_NAME: &str = "ListDir";
const GLOB_NAME: &str = "Glob";
const ASK_MODE_DISABLED: &str = "Ask mode: file modification and command execution are disabled. Use Read, Grep, ListDir, and Glob tools to explore, then respond with an explanation.";

/// Max output size for Read and Bash tool results (32 KB).
const MAX_OUTPUT_LARGE: usize = 32 * 1024;
/// Max output size for Grep, ListDir, Glob tool results (16 KB).
const MAX_OUTPUT_SMALL: usize = 16 * 1024;

/// Truncate a tool result string to the given max bytes, appending a notice.
fn truncate_tool_output(output: String, max_bytes: usize) -> String {
    if output.len() <= max_bytes {
        return output;
    }
    let total = output.len();
    // Find a safe UTF-8 boundary near max_bytes.
    let mut end = max_bytes;
    while end > 0 && !output.is_char_boundary(end) {
        end -= 1;
    }
    format!(
        "{}\n\n[... truncated, {} bytes total]",
        &output[..end],
        total
    )
}

/// Return the max output size for a given tool name, or None for unlimited.
fn output_limit_for(tool_name: &str) -> Option<usize> {
    match tool_name {
        READ_NAME | BASH_NAME => Some(MAX_OUTPUT_LARGE),
        GREP_NAME | LIST_DIR_NAME | GLOB_NAME => Some(MAX_OUTPUT_SMALL),
        _ => None,
    }
}

/// Interaction mode: "Ask" = explanations only (no write/bash), "Build" = all tools.
pub fn is_ask_mode(mode: &str) -> bool {
    mode.eq_ignore_ascii_case("ask")
}

/// Execute a single tool call. Returns `Some(ChatResult::NeedsConfirmation)` if destructive and needs confirmation.
pub(super) fn execute_tool_call(
    tool_call: &Value,
    tools_list: &[Box<dyn tools::Tool>],
    mode: &str,
    confirm_destructive: &Option<ConfirmDestructive>,
    tools_defs: &[Value],
    messages: &mut std::sync::Arc<Vec<Value>>,
    tool_log: &mut std::sync::Arc<Vec<String>>,
    on_progress: Option<&(dyn Fn(&str) + Send)>,
) -> Result<Option<ChatResult>, ChatError> {
    let id = tool_call["id"].as_str().unwrap_or_default().to_string();
    let function = &tool_call["function"];
    let name = function["name"].as_str().unwrap_or_default();
    let args_str = function["arguments"].as_str().unwrap_or("{}");

    let args: Value = serde_json::from_str(args_str).map_err(|e| ChatError::ToolArgs {
        tool: name.to_string(),
        source: e,
    })?;

    let args_preview = tools_list
        .iter()
        .find(|t| t.name() == name)
        .map(|t| t.args_preview(&args))
        .unwrap_or_default();
    let log_line = format!("→ {}: {}", name, args_preview);
    std::sync::Arc::make_mut(tool_log).push(log_line.clone());
    if let Some(ref progress) = on_progress {
        progress(&log_line);
    }

    let result = if is_ask_mode(mode) && (name == WRITE_NAME || name == BASH_NAME || name == EDIT_NAME) {
        ASK_MODE_DISABLED.to_string()
    } else {
        match tools_list.iter().find(|t| t.name() == name) {
            Some(tool) => {
                if name == BASH_NAME {
                    if let Some(command) = args.get("command").and_then(|v| v.as_str()) {
                        if tools::is_destructive(command) {
                            if let Some(cb) = confirm_destructive {
                                let confirmed = cb(command);
                                if !confirmed {
                                    "Command cancelled (destructive command not confirmed)."
                                        .to_string()
                                } else {
                                    tool.execute(&args)
                                        .unwrap_or_else(|e| format!("Error: {}", e))
                                }
                            } else {
                                return Ok(Some(ChatResult::NeedsConfirmation {
                                    command: command.to_string(),
                                    state: ConfirmState {
                                        messages: std::sync::Arc::clone(messages),
                                        tool_log: std::sync::Arc::clone(tool_log),
                                        tool_call_id: id.clone(),
                                        mode: mode.to_string(),
                                        tools: tools_defs.to_vec(),
                                        command: command.to_string(),
                                    },
                                }));
                            }
                        } else {
                            tool.execute(&args)
                                .unwrap_or_else(|e| format!("Error: {}", e))
                        }
                    } else {
                        "Error: missing command argument".to_string()
                    }
                } else {
                    tool.execute(&args)
                        .unwrap_or_else(|e| format!("Error: {}", e))
                }
            }
            None => format!("Error: unknown tool '{}'", name),
        }
    };

    // Truncate large tool outputs to stay within context budget.
    let result = match output_limit_for(name) {
        Some(limit) => truncate_tool_output(result, limit),
        None => result,
    };

    std::sync::Arc::make_mut(messages).push(json!({
        "role": "tool",
        "tool_call_id": id,
        "content": result,
    }));
    Ok(None)
}
