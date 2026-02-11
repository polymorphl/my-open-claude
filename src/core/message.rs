//! Helpers for API message content extraction.

use serde_json::Value;

/// Extract text content from an API message (user or assistant).
/// Handles both string content and array-of-blocks format.
pub fn extract_content(msg: &Value) -> Option<String> {
    let content = msg.get("content")?;
    if let Some(s) = content.as_str() {
        return Some(s.to_string());
    }
    if let Some(arr) = content.as_array() {
        for block in arr {
            if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                return Some(text.to_string());
            }
        }
    }
    None
}
