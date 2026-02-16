//! Conversation index: metadata, listing, filtering, and index mutations.

use std::collections::HashMap;
use std::io;

use serde::{Deserialize, Serialize};

use crate::core::config::Config;

use super::storage;

/// Metadata for a conversation in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMeta {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Filter conversations by title, id, or message content (case-insensitive).
/// When `content_by_id` is provided, also matches if any message content contains the query.
pub fn filter_conversations_with_content<'a>(
    convs: &'a [ConversationMeta],
    query: &str,
    content_by_id: &HashMap<String, String>,
) -> Vec<&'a ConversationMeta> {
    if query.is_empty() {
        return convs.iter().collect();
    }
    let q = query.to_lowercase();
    convs
        .iter()
        .filter(|c| {
            c.title.to_lowercase().contains(&q)
                || c.id.to_lowercase().contains(&q)
                || content_by_id
                    .get(&c.id)
                    .map(|s| s.to_lowercase().contains(&q))
                    .unwrap_or(false)
        })
        .collect()
}

/// List all conversations, sorted by updated_at descending.
pub fn list_conversations() -> io::Result<Vec<ConversationMeta>> {
    let mut index = storage::load_index()?;
    index
        .conversations
        .sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(index.conversations)
}

/// Add or update a conversation in the index. Removes any existing entry with the same id.
pub(super) fn add_or_update(meta: ConversationMeta) -> io::Result<()> {
    let mut index = storage::load_index()?;
    index.conversations.retain(|c| c.id != meta.id);
    index.conversations.push(meta);
    storage::save_index(&index)
}

/// Update the title of a conversation by ID.
pub(super) fn update_title(id: &str, new_title: &str) -> io::Result<()> {
    let mut index = storage::load_index()?;
    if let Some(meta) = index.conversations.iter_mut().find(|c| c.id == id) {
        meta.title = new_title.to_string();
        storage::save_index(&index)?;
    }
    Ok(())
}

/// Remove a conversation from the index by ID.
pub(super) fn remove(id: &str) -> io::Result<()> {
    let mut index = storage::load_index()?;
    index.conversations.retain(|c| c.id != id);
    storage::save_index(&index)
}

/// Remove old conversations when exceeding max_conversations.
/// Deletes conversation files and updates the index.
pub(super) fn prune(config: &Config) -> io::Result<()> {
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
    storage::save_index(&index)
}
