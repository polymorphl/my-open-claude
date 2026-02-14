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
pub(crate) fn truncate_tool_output(output: String, max_bytes: usize) -> String {
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
pub(crate) fn output_limit_for(tool_name: &str) -> Option<usize> {
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

/// Run a tool and format errors. Logs the underlying error before returning user-facing string.
pub(crate) fn tool_result_string(res: Result<String, tools::ToolError>, tool_name: &str) -> String {
    match res {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Tool {} error: {}", tool_name, e);
            format!("Error: {}", e)
        }
    }
}

/// Outcome of executing the Bash tool: either output string or needs user confirmation.
enum BashOutcome {
    Output(String),
    NeedsConfirmation(ConfirmState),
}

/// Execute Bash tool with destructive-command confirmation logic.
fn execute_bash_tool(
    tool: &dyn tools::Tool,
    args: &Value,
    id: &str,
    mode: &str,
    ctx: &ToolCallContext<'_>,
) -> BashOutcome {
    let command = match args.get("command").and_then(|v| v.as_str()) {
        Some(c) => c,
        None => return BashOutcome::Output("Error: missing command argument".to_string()),
    };

    if !tools::is_destructive(command) {
        return BashOutcome::Output(tool_result_string(tool.execute(args), BASH_NAME));
    }

    if let Some(cb) = ctx.confirm_destructive {
        return if cb(command) {
            BashOutcome::Output(tool_result_string(tool.execute(args), BASH_NAME))
        } else {
            BashOutcome::Output(
                "Command cancelled (destructive command not confirmed).".to_string(),
            )
        };
    }

    BashOutcome::NeedsConfirmation(ConfirmState {
        messages: std::sync::Arc::clone(ctx.messages),
        tool_log: std::sync::Arc::clone(ctx.tool_log),
        tool_call_id: id.to_string(),
        mode: mode.to_string(),
        tools: ctx.tools_defs.to_vec(),
        command: command.to_string(),
    })
}

/// Context for executing a tool call (shared state and callbacks).
pub(super) struct ToolCallContext<'a> {
    pub confirm_destructive: &'a Option<ConfirmDestructive>,
    pub tools_defs: &'a [Value],
    pub messages: &'a mut std::sync::Arc<Vec<Value>>,
    pub tool_log: &'a mut std::sync::Arc<Vec<String>>,
    pub on_progress: Option<&'a (dyn Fn(&str) + Send + Sync)>,
}

/// Execute a single tool call. Returns `Some(ChatResult::NeedsConfirmation)` if destructive and needs confirmation.
pub(super) fn execute_tool_call(
    tool_call: &Value,
    tools_list: &[Box<dyn tools::Tool>],
    mode: &str,
    ctx: &mut ToolCallContext<'_>,
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
    std::sync::Arc::make_mut(ctx.tool_log).push(log_line.clone());
    if let Some(ref progress) = ctx.on_progress {
        progress(&log_line);
    }

    let result =
        if is_ask_mode(mode) && (name == WRITE_NAME || name == BASH_NAME || name == EDIT_NAME) {
            ASK_MODE_DISABLED.to_string()
        } else {
            match tools_list.iter().find(|t| t.name() == name) {
                Some(tool) => {
                    if name == BASH_NAME {
                        match execute_bash_tool(tool.as_ref(), &args, &id, mode, ctx) {
                            BashOutcome::Output(s) => s,
                            BashOutcome::NeedsConfirmation(state) => {
                                return Ok(Some(ChatResult::NeedsConfirmation {
                                    command: state.command.clone(),
                                    state,
                                }));
                            }
                        }
                    } else {
                        tool_result_string(tool.execute(&args), name)
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

    std::sync::Arc::make_mut(ctx.messages).push(json!({
        "role": "tool",
        "tool_call_id": id,
        "content": result,
    }));
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_ask_mode_true() {
        assert!(is_ask_mode("ask"));
        assert!(is_ask_mode("Ask"));
        assert!(is_ask_mode("ASK"));
    }

    #[test]
    fn is_ask_mode_false() {
        assert!(!is_ask_mode("Build"));
        assert!(!is_ask_mode("build"));
    }

    #[test]
    fn output_limit_for_read_and_bash() {
        assert_eq!(output_limit_for("Read"), Some(32 * 1024));
        assert_eq!(output_limit_for("Bash"), Some(32 * 1024));
    }

    #[test]
    fn output_limit_for_grep_listdir_glob() {
        assert_eq!(output_limit_for("Grep"), Some(16 * 1024));
        assert_eq!(output_limit_for("ListDir"), Some(16 * 1024));
        assert_eq!(output_limit_for("Glob"), Some(16 * 1024));
    }

    #[test]
    fn output_limit_for_unknown() {
        assert_eq!(output_limit_for("Edit"), None);
        assert_eq!(output_limit_for("Write"), None);
    }

    #[test]
    fn truncate_tool_output_under_limit() {
        let s = "short output";
        assert_eq!(truncate_tool_output(s.to_string(), 100), s);
    }

    #[test]
    fn truncate_tool_output_over_limit() {
        let s = "a".repeat(50);
        let result = truncate_tool_output(s.clone(), 20);
        assert!(result.contains("[... truncated, 50 bytes total]"));
        assert!(result.len() < 50 + 35);
    }

    #[test]
    fn truncate_tool_output_utf8_boundary() {
        let s = "é".repeat(10); // 2 bytes per char
        let result = truncate_tool_output(s, 5);
        assert!(result.contains("truncated"));
    }
}
