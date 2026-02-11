//! Context window management: token estimation and message truncation.

use serde_json::{Value, json};

/// Safety margin: truncate when estimated tokens exceed this fraction of context_length.
const CONTEXT_BUDGET_RATIO: f64 = 0.85;

/// Tool names whose large arguments should be summarized in conversation history.
const WRITE_TOOL: &str = "Write";
const EDIT_TOOL: &str = "Edit";

/// Estimate the number of tokens in a single message.
/// Uses JSON byte length / 4 as a rough chars-to-tokens ratio.
fn estimate_message_tokens(msg: &Value) -> usize {
    serde_json::to_vec(msg).map(|v| v.len()).unwrap_or(0) / 4
}

/// Estimate the number of tokens in a set of messages.
///
/// Uses a conservative heuristic: JSON-serialized byte length / 4.
/// This is a rough approximation suitable for pre-call budget checks;
/// actual usage comes from the API response.
pub fn estimate_tokens(messages: &[Value]) -> usize {
    messages.iter().map(estimate_message_tokens).sum()
}

/// Truncate the oldest messages if the estimated token count exceeds the model's context budget.
///
/// Strategy:
/// - Budget = context_length * 85%
/// - Always preserve at least the last message (the current user prompt)
/// - Remove the oldest messages first (index 0, 1, ...) until under budget
///
/// Runs in O(n): computes per-message sizes once, then subtracts when removing.
pub fn truncate_if_needed(messages: &mut Vec<Value>, context_length: u64) {
    if context_length == 0 {
        return;
    }

    let budget = (context_length as f64 * CONTEXT_BUDGET_RATIO) as usize;

    // Precompute token estimate per message (O(n) once).
    let mut sizes: Vec<usize> = messages.iter().map(estimate_message_tokens).collect();
    let mut total: usize = sizes.iter().sum();

    if total <= budget || messages.len() <= 1 {
        return;
    }

    // Remove from front, subtracting from total (O(1) per removal).
    // Preserve the system message (index 0) so the model always knows the CWD.
    let remove_from = if messages
        .first()
        .and_then(|m| m.get("role").and_then(|r| r.as_str()))
        == Some("system")
    {
        1
    } else {
        0
    };
    while messages.len() > 1 && total > budget {
        if remove_from >= messages.len() {
            break;
        }
        let removed = sizes.remove(remove_from);
        total = total.saturating_sub(removed);
        messages.remove(remove_from);
    }
}

/// Summarize Write/Edit tool call arguments in an assistant message to reduce context size.
///
/// For Write tool calls: replace the `content` argument with `"[N bytes written]"`.
/// For Edit tool calls: replace `new_string` and `old_string` arguments with `"[N bytes]"`.
///
/// Call this on the last assistant message right after appending it to `messages`.
pub fn summarize_write_args_in_last(messages: &mut Vec<Value>) {
    let Some(last) = messages.last_mut() else {
        return;
    };
    if last.get("role").and_then(|r| r.as_str()) != Some("assistant") {
        return;
    }
    let Some(tool_calls) = last.get_mut("tool_calls").and_then(|v| v.as_array_mut()) else {
        return;
    };

    for tc in tool_calls.iter_mut() {
        let name = tc
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or("");

        if name != WRITE_TOOL && name != EDIT_TOOL {
            continue;
        }

        let args_str = tc
            .get("function")
            .and_then(|f| f.get("arguments"))
            .and_then(|a| a.as_str())
            .unwrap_or("{}");

        let Ok(mut args_val) = serde_json::from_str::<Value>(args_str) else {
            continue;
        };

        if name == WRITE_TOOL {
            if let Some(content) = args_val.get("content").and_then(|c| c.as_str()) {
                let len = content.len();
                args_val["content"] = json!(format!("[{} bytes written]", len));
            }
        } else if name == EDIT_TOOL {
            if let Some(new_str) = args_val.get("new_string").and_then(|c| c.as_str()) {
                let len = new_str.len();
                args_val["new_string"] = json!(format!("[{} bytes]", len));
            }
            if let Some(old_str) = args_val.get("old_string").and_then(|c| c.as_str()) {
                let len = old_str.len();
                args_val["old_string"] = json!(format!("[{} bytes]", len));
            }
        }

        // Re-serialize the modified arguments back.
        if let Ok(new_args_str) = serde_json::to_string(&args_val) {
            tc["function"]["arguments"] = json!(new_args_str);
        }
    }
}
