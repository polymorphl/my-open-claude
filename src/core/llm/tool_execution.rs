//! Execute a single tool call from the agent loop.

use serde_json::{Value, json};

use crate::core::confirm::ConfirmDestructive;
use crate::core::tools;

use super::ChatError;
use super::ChatResult;
use super::ConfirmState;

const WRITE_NAME: &str = "Write";
const BASH_NAME: &str = "Bash";
const ASK_MODE_DISABLED: &str = "Ask mode: file creation/modification and command execution are disabled. Use only the Read tool to read files, then respond with an explanation.";

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

    let result = if is_ask_mode(mode) && (name == WRITE_NAME || name == BASH_NAME) {
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

    std::sync::Arc::make_mut(messages).push(json!({
        "role": "tool",
        "tool_call_id": id,
        "content": result,
    }));
    Ok(None)
}
