//! Undo stack for file modifications made by the agent.
//!
//! Before each Write or Edit tool execution, the original file content is captured.
//! The user can then undo the last batch of changes (one agent loop iteration).

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Snapshot of files before a batch of tool calls (one agent loop iteration).
#[derive(Debug, Clone, Default)]
pub struct UndoBatch {
    /// file_path → original content (`None` if the file did not exist before).
    snapshots: HashMap<PathBuf, Option<String>>,
}

impl UndoBatch {
    /// Capture the current state of a file before it is modified.
    /// Only captures each path once per batch (first write wins).
    pub fn capture(&mut self, path: &str) {
        let path_buf = PathBuf::from(path);
        if self.snapshots.contains_key(&path_buf) {
            return; // already captured in this batch
        }
        let original = fs::read_to_string(&path_buf).ok();
        self.snapshots.insert(path_buf, original);
    }

    /// Whether any files were captured in this batch.
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }
}

/// Stack of undo batches. Each agent loop iteration that modifies files creates one batch.
#[derive(Debug, Default)]
pub struct UndoStack {
    batches: Vec<UndoBatch>,
}

impl UndoStack {
    /// Push a completed batch onto the stack (only if it contains snapshots).
    pub fn push_batch(&mut self, batch: UndoBatch) {
        if !batch.is_empty() {
            self.batches.push(batch);
        }
    }

    /// Pop and restore the last batch. Returns the number of files restored, or `None` if empty.
    pub fn undo_last(&mut self) -> Option<UndoResult> {
        let batch = self.batches.pop()?;
        let mut restored = 0;
        let mut deleted = 0;
        let mut errors: Vec<String> = Vec::new();

        for (path, original) in &batch.snapshots {
            match original {
                Some(content) => {
                    if let Err(e) = fs::write(path, content) {
                        errors.push(format!("{}: {}", path.display(), e));
                    } else {
                        restored += 1;
                    }
                }
                None => {
                    // File did not exist before — remove it.
                    if path.exists() {
                        if let Err(e) = fs::remove_file(path) {
                            errors.push(format!("{}: {}", path.display(), e));
                        } else {
                            deleted += 1;
                        }
                    }
                }
            }
        }

        Some(UndoResult {
            restored,
            deleted,
            errors,
        })
    }

    /// Number of undo batches available.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.batches.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.batches.is_empty()
    }
}

/// Result of an undo operation.
#[derive(Debug)]
pub struct UndoResult {
    /// Number of files restored to their previous content.
    pub restored: usize,
    /// Number of files deleted (they didn't exist before the agent created them).
    pub deleted: usize,
    /// Errors encountered during restoration.
    pub errors: Vec<String>,
}

impl std::fmt::Display for UndoResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = Vec::new();
        if self.restored > 0 {
            parts.push(format!(
                "{} file{} restored",
                self.restored,
                if self.restored == 1 { "" } else { "s" }
            ));
        }
        if self.deleted > 0 {
            parts.push(format!(
                "{} file{} removed",
                self.deleted,
                if self.deleted == 1 { "" } else { "s" }
            ));
        }
        if !self.errors.is_empty() {
            parts.push(format!(
                "{} error{}",
                self.errors.len(),
                if self.errors.len() == 1 { "" } else { "s" }
            ));
        }
        if parts.is_empty() {
            write!(f, "Nothing to undo")
        } else {
            write!(f, "Undo: {}", parts.join(", "))
        }
    }
}

/// Thread-safe shared undo stack.
pub type SharedUndoStack = Arc<Mutex<UndoStack>>;

/// Create a new shared undo stack.
pub fn new_shared() -> SharedUndoStack {
    Arc::new(Mutex::new(UndoStack::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_and_undo_existing_file() {
        let file = tempfile::NamedTempFile::new().expect("temp file");
        let path = file.path().to_str().expect("path").to_string();
        fs::write(&path, "original content").expect("write");

        let mut batch = UndoBatch::default();
        batch.capture(&path);

        // Simulate agent modifying the file.
        fs::write(&path, "modified content").expect("write");
        assert_eq!(fs::read_to_string(&path).unwrap(), "modified content");

        let mut stack = UndoStack::default();
        stack.push_batch(batch);
        assert_eq!(stack.len(), 1);

        let result = stack.undo_last().expect("undo");
        assert_eq!(result.restored, 1);
        assert_eq!(result.deleted, 0);
        assert!(result.errors.is_empty());
        assert_eq!(fs::read_to_string(&path).unwrap(), "original content");
    }

    #[test]
    fn capture_and_undo_new_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("new_file.txt");
        let path_str = path.to_str().expect("path").to_string();

        // File does not exist yet.
        assert!(!path.exists());

        let mut batch = UndoBatch::default();
        batch.capture(&path_str);

        // Simulate agent creating the file.
        fs::write(&path, "new content").expect("write");
        assert!(path.exists());

        let mut stack = UndoStack::default();
        stack.push_batch(batch);

        let result = stack.undo_last().expect("undo");
        assert_eq!(result.deleted, 1);
        assert_eq!(result.restored, 0);
        assert!(!path.exists());
    }

    #[test]
    fn empty_batch_not_pushed() {
        let mut stack = UndoStack::default();
        stack.push_batch(UndoBatch::default());
        assert!(stack.is_empty());
    }

    #[test]
    fn undo_empty_stack_returns_none() {
        let mut stack = UndoStack::default();
        assert!(stack.undo_last().is_none());
    }

    #[test]
    fn capture_same_path_twice_keeps_first() {
        let file = tempfile::NamedTempFile::new().expect("temp file");
        let path = file.path().to_str().expect("path").to_string();
        fs::write(&path, "v1").expect("write");

        let mut batch = UndoBatch::default();
        batch.capture(&path);

        // Simulate first modification.
        fs::write(&path, "v2").expect("write");
        // Capture again — should keep "v1", not "v2".
        batch.capture(&path);

        fs::write(&path, "v3").expect("write");

        let mut stack = UndoStack::default();
        stack.push_batch(batch);

        stack.undo_last().expect("undo");
        assert_eq!(fs::read_to_string(&path).unwrap(), "v1");
    }

    #[test]
    fn multiple_batches_undo_in_order() {
        let file = tempfile::NamedTempFile::new().expect("temp file");
        let path = file.path().to_str().expect("path").to_string();
        fs::write(&path, "original").expect("write");

        let mut batch1 = UndoBatch::default();
        batch1.capture(&path);
        fs::write(&path, "after batch1").expect("write");

        let mut batch2 = UndoBatch::default();
        batch2.capture(&path);
        fs::write(&path, "after batch2").expect("write");

        let mut stack = UndoStack::default();
        stack.push_batch(batch1);
        stack.push_batch(batch2);

        // Undo batch2 first.
        stack.undo_last().expect("undo");
        assert_eq!(fs::read_to_string(&path).unwrap(), "after batch1");

        // Undo batch1.
        stack.undo_last().expect("undo");
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    }
}
