//! Streaming chat response: tool call delta merging, size limits, token usage.

use serde_json::{json, Value};

/// Max tool calls to accept from a single response (guards against malformed API).
pub(super) const MAX_TOOL_CALLS: usize = 64;
/// Max content size (2MB) to prevent unbounded memory growth from malformed streams.
pub(super) const MAX_CONTENT_BYTES: usize = 2 * 1024 * 1024;
/// Max size for a single tool call's arguments JSON (64KB).
pub(super) const MAX_TOOL_CALL_ARGS_BYTES: usize = 64 * 1024;

/// Token usage reported by the API in the final streaming chunk.
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

impl TokenUsage {
    /// Build an estimated TokenUsage from messages (heuristic: JSON bytes / 4).
    /// Used when loading a conversation from history, before any API call.
    pub fn estimated_from_messages(messages: &[serde_json::Value]) -> Self {
        let total = super::context::estimate_tokens(messages) as u64;
        Self {
            prompt_tokens: total,
            completion_tokens: 0,
            total_tokens: total,
        }
    }
}

/// Parse token usage from a streaming chunk's `usage` field (present in the final chunk).
pub(crate) fn parse_usage(chunk: &Value) -> Option<TokenUsage> {
    let usage = chunk.get("usage")?;
    Some(TokenUsage {
        prompt_tokens: usage.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        completion_tokens: usage.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        total_tokens: usage.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
    })
}

/// Merge a tool_calls delta into accumulated tool calls (by index). Arguments are concatenated.
/// Skips deltas with out-of-bounds index to handle malformed API responses.
pub(crate) fn merge_tool_call_delta(acc: &mut Vec<Value>, delta_tc: &Value) {
    let index = match delta_tc["index"].as_u64() {
        Some(i) if i < MAX_TOOL_CALLS as u64 => i as usize,
        _ => return,
    };
    while acc.len() <= index {
        acc.push(json!({
            "id": "",
            "type": "function",
            "function": { "name": "", "arguments": "" }
        }));
    }
    let entry = &mut acc[index];
    if let Some(id) = delta_tc["id"].as_str() {
        if !id.is_empty() {
            entry["id"] = json!(id);
        }
    }
    if let Some(fn_part) = delta_tc.get("function") {
        if let Some(name) = fn_part["name"].as_str() {
            if !name.is_empty() {
                entry["function"]["name"] = json!(name);
            }
        }
        if let Some(args) = fn_part["arguments"].as_str() {
            if !args.is_empty() {
                let current = entry["function"]["arguments"].as_str().unwrap_or("");
                if current.len() + args.len() <= MAX_TOOL_CALL_ARGS_BYTES {
                    entry["function"]["arguments"] = json!(format!("{}{}", current, args));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_usage_valid_chunk() {
        let chunk = serde_json::json!({
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        });
        let usage = parse_usage(&chunk).unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 20);
        assert_eq!(usage.total_tokens, 30);
    }

    #[test]
    fn parse_usage_partial_usage() {
        let chunk = serde_json::json!({
            "usage": {
                "prompt_tokens": 5
            }
        });
        let usage = parse_usage(&chunk).unwrap();
        assert_eq!(usage.prompt_tokens, 5);
        assert_eq!(usage.completion_tokens, 0);
        assert_eq!(usage.total_tokens, 0);
    }

    #[test]
    fn parse_usage_no_usage_field() {
        let chunk = serde_json::json!({"choices": []});
        assert!(parse_usage(&chunk).is_none());
    }

    #[test]
    fn merge_tool_call_delta_index_inbounds() {
        let mut acc = vec![];
        let delta = serde_json::json!({
            "index": 0,
            "id": "call_123",
            "function": {
                "name": "Read",
                "arguments": "{\"path\": \"/tmp/foo\"}"
            }
        });
        merge_tool_call_delta(&mut acc, &delta);
        assert_eq!(acc.len(), 1);
        assert_eq!(acc[0]["id"], "call_123");
        assert_eq!(acc[0]["function"]["name"], "Read");
        assert_eq!(acc[0]["function"]["arguments"], "{\"path\": \"/tmp/foo\"}");
    }

    #[test]
    fn merge_tool_call_delta_concatenates_arguments() {
        let mut acc = vec![];
        let delta1 = serde_json::json!({
            "index": 0,
            "function": {"arguments": "{\"path\": \""}
        });
        let delta2 = serde_json::json!({
            "index": 0,
            "function": {"arguments": "/tmp/foo\"}"}
        });
        merge_tool_call_delta(&mut acc, &delta1);
        merge_tool_call_delta(&mut acc, &delta2);
        assert_eq!(acc[0]["function"]["arguments"], "{\"path\": \"/tmp/foo\"}");
    }

    #[test]
    fn merge_tool_call_delta_out_of_bounds_index_skipped() {
        let mut acc = vec![];
        let delta = serde_json::json!({
            "index": 100,
            "function": {"name": "Read", "arguments": "{}"}
        });
        merge_tool_call_delta(&mut acc, &delta);
        assert!(acc.is_empty());
    }
}
