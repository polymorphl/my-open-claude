//! History module tests.

use std::io;

use serde_json::Value;

use crate::core::config::Config;
use std::collections::HashMap;

use crate::core::history::index::ConversationMeta;
use crate::core::history::{
    filter_conversations_with_content, first_message_preview, load_conversation, save_conversation,
};
use async_openai::config::OpenAIConfig;

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
    let cache = HashMap::new();
    let out = filter_conversations_with_content(&convs, "", &cache);
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
    let cache = HashMap::new();
    let out = filter_conversations_with_content(&convs, "hello", &cache);
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
    let cache = HashMap::new();
    let out = filter_conversations_with_content(&convs, "abc", &cache);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].id, "abc-123");
}

#[test]
fn filter_conversations_match_by_content() {
    let convs = vec![
        ConversationMeta {
            id: "1".to_string(),
            title: "Chat A".to_string(),
            created_at: 0,
            updated_at: 0,
        },
        ConversationMeta {
            id: "2".to_string(),
            title: "Chat B".to_string(),
            created_at: 0,
            updated_at: 0,
        },
    ];
    let mut cache = HashMap::new();
    cache.insert(
        "2".to_string(),
        "Detailed discussion about Rust ownership".to_string(),
    );
    let out = filter_conversations_with_content(&convs, "Rust", &cache);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].id, "2");
}

fn test_config() -> Config {
    Config {
        openai_config: OpenAIConfig::new(),
        model_id: "test".to_string(),
        base_url: "https://test".to_string(),
        api_key: "test".to_string(),
        max_conversations: 10,
    }
}

static PERSISTENCE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct EnvGuard(&'static str);
impl Drop for EnvGuard {
    fn drop(&mut self) {
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

    let id = save_conversation(None, "Test Chat", &messages, &config).expect("save should succeed");
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

    let config = test_config();
    let messages = vec![serde_json::json!({"role": "user", "content": "Hi"})];
    let id = save_conversation(None, "Title", &messages, &config).expect("save ok");

    let conv_path = data_dir.join(format!("conv_{}.json", id));
    std::fs::write(&conv_path, "not valid json {{{").expect("write");

    let loaded = load_conversation(&id);
    assert!(loaded.is_none());
}
