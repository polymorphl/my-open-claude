//! Persistence of conversation history in ~/.local/share/my-open-claude/conversations/.

mod storage;

use std::io;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::core::config::Config;
use crate::core::message;
use crate::core::util;

/// Metadata for a conversation in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMeta {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
}

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

/// Generate title from first user message. Truncates to max_len with ellipsis.
pub fn first_message_preview(messages: &[Value], max_len: usize) -> String {
    for msg in messages {
        if msg.get("role").and_then(|r| r.as_str()) == Some("user")
            && let Some(content) = message::extract_content(msg)
        {
            let s = content.trim().replace('\n', " ");
            if s.len() <= max_len {
                return s;
            }
            return format!("{}…", &s[..max_len.saturating_sub(1)]);
        }
    }
    "(No title)".to_string()
}

/// Filter conversations by title or id (case-insensitive).
pub fn filter_conversations<'a>(
    convs: &'a [ConversationMeta],
    query: &str,
) -> Vec<&'a ConversationMeta> {
    util::filter_by_query(convs, query, |c| (c.title.as_str(), c.id.as_str()))
}

/// List all conversations, sorted by updated_at descending.
pub fn list_conversations() -> io::Result<Vec<ConversationMeta>> {
    let mut index = storage::load_index()?;
    index
        .conversations
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(index.conversations)
}

/// Load a conversation by ID. Returns the messages in API format.
pub fn load_conversation(id: &str) -> Option<Vec<Value>> {
    storage::read_conv_messages(id)
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

    let mut index = storage::load_index()?;
    let created_at = id
        .and_then(|existing_id| {
            index
                .conversations
                .iter()
                .find(|c| c.id == existing_id)
                .map(|c| c.created_at)
        })
        .unwrap_or(now);

    let meta = ConversationMeta {
        id: conv_id.clone(),
        title: title.to_string(),
        created_at,
        updated_at: now,
    };

    index.conversations.retain(|c| c.id != conv_id);
    index.conversations.push(meta);
    storage::save_index(&index)?;

    prune_if_needed(config)?;
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
    let mut index = storage::load_index()?;
    if let Some(meta) = index.conversations.iter_mut().find(|c| c.id == id) {
        meta.title = new_title.to_string();
        storage::save_index(&index)?;
    }
    Ok(())
}

/// Delete a conversation by ID. Removes the file and index entry.
pub fn delete_conversation(id: &str) -> io::Result<()> {
    storage::remove_conv_file(id)?;
    let mut index = storage::load_index()?;
    index.conversations.retain(|c| c.id != id);
    storage::save_index(&index)?;
    Ok(())
}

/// Remove old conversations when exceeding max_conversations.
pub fn prune_if_needed(config: &Config) -> io::Result<()> {
    let max = config.max_conversations as usize;
    if max == 0 {
        return Ok(());
    }

    let mut index = storage::load_index()?;
    index
        .conversations
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    if index.conversations.len() <= max {
        return Ok(());
    }

    let to_remove: Vec<_> = index.conversations.drain(max..).collect();
    for meta in &to_remove {
        if let Err(e) = storage::remove_conv_file(&meta.id) {
            log::warn!("Failed to remove conversation file {}: {}", meta.id, e);
        }
    }
    storage::save_index(&index)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io;

    use crate::core::config::Config;
    use async_openai::config::OpenAIConfig;

    use super::*;

    #[test]
    fn first_message_preview_empty_messages() {
        let messages: Vec<Value> = vec![];
        assert_eq!(first_message_preview(&messages, 50), "(No title)");
    }

    #[test]
    fn first_message_preview_no_user_message() {
        let messages = vec![serde_json::json!({"role": "assistant", "content": "Hi"})];
        assert_eq!(first_message_preview(&messages, 50), "(No title)");
    }

    #[test]
    fn first_message_preview_with_user_message() {
        let messages = vec![
            serde_json::json!({"role": "system", "content": "You are helpful"}),
            serde_json::json!({"role": "user", "content": "Hello world"}),
        ];
        assert_eq!(first_message_preview(&messages, 50), "Hello world");
    }

    #[test]
    fn first_message_preview_truncates_long_message() {
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": "This is a very long message that exceeds the max length"
        })];
        let result = first_message_preview(&messages, 20);
        assert!(result.ends_with('…'));
        assert!(result.starts_with("This is a very long"));
    }

    #[test]
    fn first_message_preview_trims_and_replaces_newlines() {
        let messages = vec![serde_json::json!({
            "role": "user",
            "content": "  Hello\nworld  "
        })];
        assert_eq!(first_message_preview(&messages, 50), "Hello world");
    }

    #[test]
    fn filter_conversations_empty_query_returns_all() {
        let convs = vec![
            ConversationMeta {
                id: "1".to_string(),
                title: "Chat 1".to_string(),
                created_at: 0,
                updated_at: 0,
            },
            ConversationMeta {
                id: "2".to_string(),
                title: "Chat 2".to_string(),
                created_at: 0,
                updated_at: 0,
            },
        ];
        let out = filter_conversations(&convs, "");
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn filter_conversations_match_by_title() {
        let convs = vec![
            ConversationMeta {
                id: "1".to_string(),
                title: "Hello world".to_string(),
                created_at: 0,
                updated_at: 0,
            },
            ConversationMeta {
                id: "2".to_string(),
                title: "Other chat".to_string(),
                created_at: 0,
                updated_at: 0,
            },
        ];
        let out = filter_conversations(&convs, "hello");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].title, "Hello world");
    }

    #[test]
    fn filter_conversations_match_by_id() {
        let convs = vec![ConversationMeta {
            id: "abc-123".to_string(),
            title: "Chat".to_string(),
            created_at: 0,
            updated_at: 0,
        }];
        let out = filter_conversations(&convs, "abc");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "abc-123");
    }

    /// Config for tests (no API key needed for save/load).
    fn test_config() -> Config {
        Config {
            openai_config: OpenAIConfig::new(),
            model_id: "test".to_string(),
            base_url: "https://test".to_string(),
            api_key: "test".to_string(),
            max_conversations: 10,
        }
    }

    /// Serializes persistence tests that use the global TEST_DATA_DIR env var.
    static PERSISTENCE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct EnvGuard(&'static str);
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: Test env isolation; no other threads access this var during test.
            unsafe {
                std::env::remove_var(self.0);
            }
        }
    }

    #[test]
    fn save_conversation_empty_messages_new_returns_err() {
        let _lock = PERSISTENCE_TEST_LOCK.lock().unwrap();
        let tmp = tempfile::TempDir::new().expect("temp dir");
        unsafe {
            std::env::set_var("TEST_DATA_DIR", tmp.path().join("conversations"));
        }
        let _guard = EnvGuard("TEST_DATA_DIR");

        let config = test_config();
        let messages: Vec<Value> = vec![];
        let result = save_conversation(None, "title", &messages, &config);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn save_then_load_roundtrip() {
        let _lock = PERSISTENCE_TEST_LOCK.lock().unwrap();
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let data_dir = tmp.path().join("conversations");
        unsafe {
            std::env::set_var("TEST_DATA_DIR", &data_dir);
        }
        let _guard = EnvGuard("TEST_DATA_DIR");

        let config = test_config();
        let messages = vec![
            serde_json::json!({"role": "user", "content": "Hello"}),
            serde_json::json!({"role": "assistant", "content": "Hi"}),
        ];

        let id =
            save_conversation(None, "Test Chat", &messages, &config).expect("save should succeed");
        assert!(!id.is_empty());

        let loaded = load_conversation(&id).expect("load should return Some");
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0]["role"], "user");
        assert_eq!(loaded[1]["role"], "assistant");
    }

    #[test]
    fn load_conversation_nonexistent_returns_none() {
        let _lock = PERSISTENCE_TEST_LOCK.lock().unwrap();
        let tmp = tempfile::TempDir::new().expect("temp dir");
        unsafe {
            std::env::set_var("TEST_DATA_DIR", tmp.path().join("conversations"));
        }
        let _guard = EnvGuard("TEST_DATA_DIR");

        let loaded = load_conversation("nonexistent-id-12345");
        assert!(loaded.is_none());
    }

    #[test]
    fn load_conversation_invalid_json_returns_none() {
        let _lock = PERSISTENCE_TEST_LOCK.lock().unwrap();
        let tmp = tempfile::TempDir::new().expect("temp dir");
        let data_dir = tmp.path().join("conversations");
        std::fs::create_dir_all(&data_dir).expect("create dir");
        unsafe {
            std::env::set_var("TEST_DATA_DIR", &data_dir);
        }
        let _guard = EnvGuard("TEST_DATA_DIR");

        // Save valid conversation first
        let config = test_config();
        let messages = vec![serde_json::json!({"role": "user", "content": "Hi"})];
        let id = save_conversation(None, "Title", &messages, &config).expect("save ok");

        // Corrupt the file with invalid JSON
        let conv_path = data_dir.join(format!("conv_{}.json", id));
        std::fs::write(&conv_path, "not valid json {{{").expect("write");

        let loaded = load_conversation(&id);
        assert!(loaded.is_none());
    }
}
