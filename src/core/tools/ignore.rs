//! Smart-ignore helper for directory traversal.
//!
//! Filters out common junk directories (node_modules, target, .git, etc.)
//! used by Grep, ListDir, and Glob tools.

/// Directories always skipped during traversal.
const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "__pycache__",
    ".venv",
    "dist",
    "build",
    ".next",
    ".cache",
];

/// Returns `true` if this directory entry should be pruned from traversal.
pub fn is_ignored(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_dir()
        && entry
            .file_name()
            .to_str()
            .is_some_and(|n| IGNORED_DIRS.contains(&n))
}
