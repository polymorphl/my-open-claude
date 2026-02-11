//! Persistence of conversation history in ~/.local/share/my-open-claude/conversations/.

use std::fs;
use std::io;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::core::config::Config;
use crate::core::message;
use crate::core::paths;
use crate::core::util;

fn index_path() -> Option<std::path::PathBuf> {
    paths::data_dir().map(|d| d.join("index.json"))
}

fn conv_path(id: &str) -> Option<std::path::PathBuf> {
    paths::data_dir().map(|d| d.join(format!("conv_{}.json", id)))
}

/// Metadata for a conversation in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMeta {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexFile {
    conversations: Vec<ConversationMeta>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConvFile {
    messages: Vec<Value>,
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
        if msg.get("role").and_then(|r| r.as_str()) == Some("user") {
            if let Some(content) = message::extract_content(msg) {
                let s = content.trim().replace('\n', " ");
                if s.len() <= max_len {
                    return s;
                }
                return format!("{}…", &s[..max_len.saturating_sub(1)]);
            }
        }
    }
    "(No title)".to_string()
}

fn ensure_data_dir() -> io::Result<std::path::PathBuf> {
    let dir = paths::data_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No data directory"))?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn load_index() -> IndexFile {
    let path = match index_path() {
        Some(p) => p,
        None => {
            return IndexFile {
                conversations: vec![],
            };
        }
    };
    let data = match fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Warning: could not read history index at {:?}: {}", path, e);
            return IndexFile {
                conversations: vec![],
            };
        }
    };
    match serde_json::from_str(&data) {
        Ok(index) => index,
        Err(e) => {
            eprintln!(
                "Warning: invalid JSON in history index at {:?}: {}",
                path, e
            );
            IndexFile {
                conversations: vec![],
            }
        }
    }
}

fn save_index(index: &IndexFile) -> io::Result<()> {
    ensure_data_dir()?;
    let path =
        index_path().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No index path"))?;
    let json = serde_json::to_string_pretty(index)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, json)?;
    fs::rename(tmp, path)?;
    Ok(())
}

/// Filter conversations by title or id (case-insensitive).
pub fn filter_conversations<'a>(
    convs: &'a [ConversationMeta],
    query: &str,
) -> Vec<&'a ConversationMeta> {
    util::filter_by_query(convs, query, |c| (c.title.as_str(), c.id.as_str()))
}

/// List all conversations, sorted by updated_at descending.
pub fn list_conversations() -> Vec<ConversationMeta> {
    let mut index = load_index();
    index
        .conversations
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    index.conversations
}

/// Load a conversation by ID. Returns the messages in API format.
pub fn load_conversation(id: &str) -> Option<Vec<Value>> {
    let path = conv_path(id)?;
    let data = fs::read_to_string(path).ok()?;
    let file: ConvFile = serde_json::from_str(&data).ok()?;
    Some(file.messages)
}

/// Save a conversation. Creates or updates. Returns the conversation ID.
pub fn save_conversation(
    id: Option<&str>,
    title: &str,
    messages: &[Value],
    config: &Config,
) -> io::Result<String> {
    ensure_data_dir()?;
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
        .unwrap_or(0);

    let conv_id = id
        .map(String::from)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let path = conv_path(&conv_id)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No conv path"))?;

    let file = ConvFile {
        messages: sanitized.clone(),
    };
    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, json)?;
    fs::rename(tmp, &path)?;

    let mut index = load_index();
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
    save_index(&index)?;

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
    let mut index = load_index();
    if let Some(meta) = index.conversations.iter_mut().find(|c| c.id == id) {
        meta.title = new_title.to_string();
        save_index(&index)?;
    }
    Ok(())
}

/// Delete a conversation by ID. Removes the file and index entry.
pub fn delete_conversation(id: &str) -> io::Result<()> {
    if let Some(p) = conv_path(id) {
        let _ = fs::remove_file(p);
    }
    let mut index = load_index();
    index.conversations.retain(|c| c.id != id);
    save_index(&index)?;
    Ok(())
}

/// Remove old conversations when exceeding max_conversations.
pub fn prune_if_needed(config: &Config) -> io::Result<()> {
    let max = config.max_conversations as usize;
    if max == 0 {
        return Ok(());
    }

    let mut index = load_index();
    index
        .conversations
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    if index.conversations.len() <= max {
        return Ok(());
    }

    let to_remove: Vec<_> = index.conversations.drain(max..).collect();
    for meta in &to_remove {
        if let Some(p) = conv_path(&meta.id) {
            let _ = fs::remove_file(p);
        }
    }
    save_index(&index)?;
    Ok(())
}
