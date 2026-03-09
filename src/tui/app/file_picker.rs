//! File picker popup state for @ reference insertion.

use ratatui::widgets::ListState;
use std::path::PathBuf;

use crate::core::tools::ignore::is_ignored_name;

/// A single directory entry shown in the file picker.
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub rel_path: PathBuf,
}

/// State for the file picker popup (opened when user types `@` at start or after whitespace).
pub struct FilePickerState {
    pub current_dir: PathBuf,
    pub all_entries: Vec<DirEntry>,
    pub filter: String,
    pub list_state: ListState,
    pub selected_index: usize,
    pub workspace_root: PathBuf,
    /// Byte offset of `@` in app.input.
    pub at_token_start: usize,
}

impl FilePickerState {
    /// Open the file picker rooted at `dir`. `workspace_root` is the upper boundary for ascend().
    pub fn open(dir: PathBuf, workspace_root: PathBuf, at_token_start: usize) -> Self {
        let mut state = Self {
            current_dir: dir,
            all_entries: Vec::new(),
            filter: String::new(),
            list_state: ListState::default(),
            selected_index: 0,
            workspace_root,
            at_token_start,
        };
        state.load_entries();
        state
    }

    /// Return entries whose names start with the current filter (case-insensitive).
    pub fn filtered_entries(&self) -> Vec<&DirEntry> {
        let filter_lower = self.filter.to_lowercase();
        self.all_entries
            .iter()
            .filter(|e| e.name.to_lowercase().starts_with(&filter_lower))
            .collect()
    }

    /// Descend into a subdirectory and reset the filter.
    pub fn descend(&mut self, name: &str) {
        self.current_dir = self.current_dir.join(name);
        self.filter.clear();
        self.reload();
    }

    /// Ascend to the parent directory, unless already at workspace_root.
    pub fn ascend(&mut self) {
        if self.current_dir <= self.workspace_root {
            return;
        }
        if let Some(parent) = self.current_dir.parent() {
            self.current_dir = parent.to_path_buf();
            self.filter.clear();
            self.reload();
        }
    }

    /// Reload entries and reset selection.
    pub fn reload(&mut self) {
        self.load_entries();
        self.selected_index = 0;
        self.list_state.select(Some(0));
    }

    /// Read the current directory: dirs alphabetically first, then files; skip hidden and ignored names.
    pub fn load_entries(&mut self) {
        self.all_entries.clear();

        let read = match std::fs::read_dir(&self.current_dir) {
            Ok(r) => r,
            Err(_) => return,
        };

        let mut dirs: Vec<DirEntry> = Vec::new();
        let mut files: Vec<DirEntry> = Vec::new();

        for entry in read.flatten() {
            let name = match entry.file_name().into_string() {
                Ok(n) => n,
                Err(_) => continue,
            };

            // Skip hidden names (starting with '.')
            if name.starts_with('.') {
                continue;
            }

            // Skip names that the ignore filter rejects
            if is_ignored_name(&name) {
                continue;
            }

            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

            let rel_path = match self.current_dir.strip_prefix(&self.workspace_root) {
                Ok(rel) => rel.join(&name),
                Err(_) => PathBuf::from(&name),
            };

            let de = DirEntry {
                name,
                is_dir,
                rel_path,
            };
            if is_dir {
                dirs.push(de);
            } else {
                files.push(de);
            }
        }

        dirs.sort_by(|a, b| a.name.cmp(&b.name));
        files.sort_by(|a, b| a.name.cmp(&b.name));

        self.all_entries.extend(dirs);
        self.all_entries.extend(files);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn make_tmp() -> tempfile::TempDir {
        tempfile::tempdir().expect("tmp dir")
    }

    #[test]
    fn open_populates_all_entries() {
        let tmp = make_tmp();
        let root = tmp.path().to_path_buf();

        // Create some files and a subdir
        fs::write(root.join("foo.rs"), "").unwrap();
        fs::write(root.join("bar.txt"), "").unwrap();
        fs::create_dir(root.join("src")).unwrap();

        let picker = FilePickerState::open(root.clone(), root, 0);
        assert!(
            !picker.all_entries.is_empty(),
            "entries should be populated"
        );
        let names: Vec<&str> = picker.all_entries.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"src"));
        assert!(names.contains(&"foo.rs"));
        assert!(names.contains(&"bar.txt"));
    }

    #[test]
    fn filtered_entries_prefix_match() {
        let tmp = make_tmp();
        let root = tmp.path().to_path_buf();

        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src_test.rs"), "").unwrap();
        fs::write(root.join("main.rs"), "").unwrap();

        let mut picker = FilePickerState::open(root.clone(), root, 0);
        picker.filter = "src".to_string();

        let filtered = picker.filtered_entries();
        assert_eq!(filtered.len(), 2);
        for e in &filtered {
            assert!(e.name.starts_with("src"));
        }
    }

    #[test]
    fn ascend_at_workspace_root_is_noop() {
        let tmp = make_tmp();
        let root = tmp.path().to_path_buf();
        let mut picker = FilePickerState::open(root.clone(), root.clone(), 0);
        let before = picker.current_dir.clone();
        picker.ascend();
        assert_eq!(
            picker.current_dir, before,
            "should not ascend past workspace_root"
        );
    }

    #[test]
    fn descend_updates_current_dir() {
        let tmp = make_tmp();
        let root = tmp.path().to_path_buf();
        fs::create_dir(root.join("subdir")).unwrap();

        let mut picker = FilePickerState::open(root.clone(), root.clone(), 0);
        picker.descend("subdir");
        assert_eq!(picker.current_dir, root.join("subdir"));
    }
}
