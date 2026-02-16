//! Persistence of conversation history in ~/.local/share/my-open-claude/conversations/.

mod index;
mod storage;

pub use index::{ConversationMeta, filter_conversations_with_content, list_conversations};

use std::io;

use serde_json::Value;
use uuid::Uuid;

use crate::core::config::Config;
use crate::core::message;

/// Extract messages suitable for persistence: only user and assistant with content.
fn sanitize_messages_for_save(messages: &[Value]) -> Vec<Value> {
    messages
        .iter()
        .filter_map(|msg| {
            let role = msg.get("role")?.as_str()?;
            match role {
                "user" => {
                    let content = msg.get("content")?;
                    Some(serde_json::json!({"role": "user", "content": content}))
                }
                "assistant" => {
                    let content = msg
                        .get("content")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    Some(serde_json::json!({"role": "assistant", "content": content}))
                }
                _ => None,
            }
        })
        .collect()
}

/// Generate title from first user message. Truncates to max_len chars with ellipsis.
pub fn first_message_preview(messages: &[Value], max_len: usize) -> String {
    for msg in messages {
        if msg.get("role").and_then(|r| r.as_str()) == Some("user")
            && let Some(content) = message::extract_content(msg)
        {
            let s = content.trim().replace('\n', " ");
            let char_count = s.chars().count();
            if char_count <= max_len {
                return s;
            }
            let truncated: String = s.chars().take(max_len.saturating_sub(1)).collect();
            return format!("{}…", truncated);
        }
    }
    "(No title)".to_string()
}

/// Load a conversation by ID. Returns the messages in API format.
pub fn load_conversation(id: &str) -> Option<Vec<Value>> {
    storage::read_conv_messages(id)
}

/// Load concatenated text content from a conversation for full-text search.
/// Returns None if the conversation cannot be loaded.
pub fn load_conversation_searchable_content(id: &str) -> Option<String> {
    let messages = storage::read_conv_messages(id)?;
    let parts: Vec<String> = messages
        .iter()
        .filter_map(message::extract_content)
        .collect();
    Some(parts.join("\n"))
}

/// Save a conversation. Creates or updates. Returns the conversation ID.
pub fn save_conversation(
    id: Option<&str>,
    title: &str,
    messages: &[Value],
    config: &Config,
) -> io::Result<String> {
    storage::ensure_data_dir()?;
    let sanitized = sanitize_messages_for_save(messages);
    if sanitized.is_empty() {
        if let Some(existing_id) = id {
            return Ok(existing_id.to_string());
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Cannot save empty conversation",
        ));
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_else(|e| {
            log::warn!("System time before UNIX epoch: {}", e);
            0
        });

    let conv_id = id
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    storage::write_conv_file(&conv_id, &sanitized)?;

    let created_at = id
        .and_then(|existing_id| {
            storage::load_index().ok().and_then(|idx| {
                idx.conversations
                    .iter()
                    .find(|c| c.id == existing_id)
                    .map(|c| c.created_at)
            })
        })
        .unwrap_or(now);

    let meta = index::ConversationMeta {
        id: conv_id.clone(),
        title: title.to_string(),
        created_at,
        updated_at: now,
    };

    index::add_or_update(meta)?;
    index::prune(config)?;
    Ok(conv_id)
}

/// Rename a conversation by ID. Updates only the title in the index.
pub fn rename_conversation(id: &str, new_title: &str) -> io::Result<()> {
    let new_title = new_title.trim();
    if new_title.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Title cannot be empty",
        ));
    }
    index::update_title(id, new_title)
}

/// Delete a conversation by ID. Removes the file and index entry.
pub fn delete_conversation(id: &str) -> io::Result<()> {
    storage::remove_conv_file(id)?;
    index::remove(id)
}

#[cfg(test)]
mod tests;
