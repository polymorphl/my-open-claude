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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_content_string_direct() {
        let msg = serde_json::json!({"role": "user", "content": "Hello world"});
        assert_eq!(extract_content(&msg), Some("Hello world".to_string()));
    }

    #[test]
    fn extract_content_array_of_blocks() {
        let msg = serde_json::json!({
            "role": "assistant",
            "content": [{"type": "text", "text": "Response text"}]
        });
        assert_eq!(extract_content(&msg), Some("Response text".to_string()));
    }

    #[test]
    fn extract_content_missing_content() {
        let msg = serde_json::json!({"role": "user"});
        assert_eq!(extract_content(&msg), None);
    }

    #[test]
    fn extract_content_empty_array() {
        let msg = serde_json::json!({"role": "assistant", "content": []});
        assert_eq!(extract_content(&msg), None);
    }

    #[test]
    fn extract_content_blocks_without_text() {
        let msg = serde_json::json!({
            "role": "assistant",
            "content": [{"type": "image"}]
        });
        assert_eq!(extract_content(&msg), None);
    }
}
