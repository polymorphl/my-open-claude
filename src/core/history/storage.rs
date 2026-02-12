//! Index and conversation file persistence (index.json, conv_*.json).

use std::fs;
use std::io;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::paths;

use super::ConversationMeta;

fn index_path() -> Option<std::path::PathBuf> {
    paths::data_dir().map(|d| d.join("index.json"))
}

fn conv_path(id: &str) -> Option<std::path::PathBuf> {
    paths::data_dir().map(|d| d.join(format!("conv_{}.json", id)))
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct IndexFile {
    pub(super) conversations: Vec<ConversationMeta>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConvFile {
    messages: Vec<Value>,
}

pub(super) fn ensure_data_dir() -> io::Result<std::path::PathBuf> {
    let dir = paths::data_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No data directory"))?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Load the conversation index. Returns empty index when no data dir or file not found (first run).
/// Propagates IO errors (permission, disk) and JSON parse errors.
pub(super) fn load_index() -> io::Result<IndexFile> {
    let path = match index_path() {
        Some(p) => p,
        None => {
            return Ok(IndexFile {
                conversations: vec![],
            });
        }
    };
    let data = match fs::read_to_string(&path) {
        Ok(d) => d,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok(IndexFile {
                conversations: vec![],
            });
        }
        Err(e) => return Err(e),
    };
    serde_json::from_str(&data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

pub(super) fn save_index(index: &IndexFile) -> io::Result<()> {
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

pub(super) fn read_conv_messages(id: &str) -> Option<Vec<Value>> {
    let path = conv_path(id)?;
    let data = fs::read_to_string(path).ok()?;
    let file: ConvFile = serde_json::from_str(&data).ok()?;
    Some(file.messages)
}

pub(super) fn write_conv_file(id: &str, messages: &[Value]) -> io::Result<()> {
    let path =
        conv_path(id).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No conv path"))?;
    let file = ConvFile {
        messages: messages.to_vec(),
    };
    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, json)?;
    fs::rename(tmp, path)?;
    Ok(())
}

pub(super) fn remove_conv_file(id: &str) {
    if let Some(p) = conv_path(id) {
        let _ = fs::remove_file(p);
    }
}
