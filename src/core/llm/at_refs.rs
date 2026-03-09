//! Resolves `@path` references in user input by appending file/dir content.
//!
//! When the user types `@src/main.rs` in their prompt, this module detects
//! such tokens and appends the referenced file (or directory listing) to the
//! prompt text, so the model receives the content inline.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

const MAX_FILE_BYTES: usize = 64 * 1024; // 64 KiB
const MAX_DIR_ENTRIES: usize = 200;

/// Resolve `@path` references in `input` relative to `workspace_root`.
///
/// Rules:
/// - A token is `@` followed by non-whitespace characters.
/// - The `@` must be at the start of the string or preceded by whitespace
///   (so `email@example.com` mid-word is NOT triggered).
/// - The resolved canonical path must be inside `workspace_root` (security).
/// - Duplicate paths are appended only once.
/// - Files are read up to 64 KiB; larger files are truncated with a notice.
/// - Binary files are noted as `[binary file: N bytes]`.
/// - Directories produce a listing of up to 200 entries.
pub fn resolve_at_refs(input: &str, workspace_root: &Path) -> String {
    let mut appended: HashSet<PathBuf> = HashSet::new();
    let mut appendix = String::new();

    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Look for '@'
        if bytes[i] == b'@' {
            // '@' must be at start or preceded by whitespace
            let preceded_by_whitespace = i == 0 || bytes[i - 1].is_ascii_whitespace();
            if preceded_by_whitespace {
                // Collect the token after '@'
                let start = i + 1;
                let mut end = start;
                while end < len && !bytes[end].is_ascii_whitespace() {
                    end += 1;
                }
                if end > start {
                    let token = &input[start..end];
                    process_at_token(token, workspace_root, &mut appended, &mut appendix);
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }

    if appendix.is_empty() {
        input.to_string()
    } else {
        format!("{}\n{}", input, appendix)
    }
}

/// Process one `@token`, resolve path, and append content to `appendix` if valid.
fn process_at_token(
    token: &str,
    workspace_root: &Path,
    appended: &mut HashSet<PathBuf>,
    appendix: &mut String,
) {
    let path = workspace_root.join(token);

    // Canonicalize to resolve `..` and symlinks
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => return, // non-existent or inaccessible — silently ignore
    };

    // Security: must remain inside workspace_root
    let canonical_root = match workspace_root.canonicalize() {
        Ok(r) => r,
        Err(_) => return,
    };
    if !canonical.starts_with(&canonical_root) {
        return; // path escapes workspace — silently ignore
    }

    // Dedup
    if appended.contains(&canonical) {
        return;
    }
    appended.insert(canonical.clone());

    if canonical.is_dir() {
        append_dir_listing(token, &canonical, appendix);
    } else {
        append_file_content(token, &canonical, appendix);
    }
}

/// Append a directory listing to `appendix`.
fn append_dir_listing(token: &str, dir: &Path, appendix: &mut String) {
    let entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };

    appendix.push_str("\n---\n");
    appendix.push_str(&format!("**@{}**\n", token));

    let mut names: Vec<String> = entries
        .iter()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();

    let shown = names.len().min(MAX_DIR_ENTRIES);
    for name in &names[..shown] {
        appendix.push_str(&format!("  {}\n", name));
    }
    if names.len() > MAX_DIR_ENTRIES {
        appendix.push_str(&format!(
            "  … ({} more entries not shown)\n",
            names.len() - MAX_DIR_ENTRIES
        ));
    }

    appendix.push_str("---\n");
}

/// Append file content (with syntax highlighting hint) to `appendix`.
fn append_file_content(token: &str, file: &Path, appendix: &mut String) {
    let raw = match std::fs::read(file) {
        Ok(b) => b,
        Err(_) => return,
    };

    let total_bytes = raw.len();

    appendix.push_str("\n---\n");
    appendix.push_str(&format!("**@{}**\n", token));

    // Detect binary: look for null bytes in first 8 KiB sample
    let sample = &raw[..raw.len().min(8192)];
    if sample.contains(&0u8) {
        appendix.push_str(&format!("[binary file: {} bytes]\n", total_bytes));
        appendix.push_str("---\n");
        return;
    }

    let truncated = total_bytes > MAX_FILE_BYTES;
    let slice = &raw[..raw.len().min(MAX_FILE_BYTES)];
    let text = String::from_utf8_lossy(slice);

    // Infer language from extension for the fenced code block
    let lang = file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();

    appendix.push_str(&format!("```{}\n", lang));
    appendix.push_str(&text);
    if !text.ends_with('\n') {
        appendix.push('\n');
    }
    appendix.push_str("```\n");

    if truncated {
        appendix.push_str(&format!(
            "[truncated: showing first {} of {} bytes]\n",
            MAX_FILE_BYTES, total_bytes
        ));
    }

    appendix.push_str("---\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_workspace() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn test_no_at_refs_unchanged() {
        let dir = make_workspace();
        let input = "hello world, no references here";
        let result = resolve_at_refs(input, dir.path());
        assert_eq!(result, input);
    }

    #[test]
    fn test_valid_file_ref_appended() {
        let dir = make_workspace();
        let file_path = dir.path().join("main.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let input = "@main.rs";
        let result = resolve_at_refs(input, dir.path());
        assert!(
            result.contains("fn main() {}"),
            "file content should appear"
        );
        assert!(result.contains("**@main.rs**"), "header should appear");
    }

    #[test]
    fn test_nonexistent_path_ignored() {
        let dir = make_workspace();
        let input = "@does_not_exist.rs";
        let result = resolve_at_refs(input, dir.path());
        // Should be unchanged — no extra content
        assert_eq!(result, input);
    }

    #[test]
    fn test_path_traversal_ignored() {
        let dir = make_workspace();
        // Attempt to escape workspace
        let input = "@../../etc/passwd";
        let result = resolve_at_refs(input, dir.path());
        assert_eq!(result, input, "path traversal should be silently ignored");
    }

    #[test]
    fn test_duplicate_ref_appended_once() {
        let dir = make_workspace();
        let file_path = dir.path().join("lib.rs");
        fs::write(&file_path, "pub fn foo() {}").unwrap();

        let input = "@lib.rs and again @lib.rs";
        let result = resolve_at_refs(input, dir.path());

        // Count occurrences of the header
        let count = result.matches("**@lib.rs**").count();
        assert_eq!(count, 1, "duplicate path should only appear once");
    }

    #[test]
    fn test_dir_ref_produces_listing() {
        let dir = make_workspace();
        let subdir = dir.path().join("src");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("main.rs"), "fn main() {}").unwrap();
        fs::write(subdir.join("lib.rs"), "pub fn foo() {}").unwrap();

        let input = "@src";
        let result = resolve_at_refs(input, dir.path());
        assert!(result.contains("**@src**"), "dir header should appear");
        assert!(result.contains("main.rs"), "dir entry should appear");
        assert!(result.contains("lib.rs"), "dir entry should appear");
    }

    #[test]
    fn test_email_mid_word_not_triggered() {
        let dir = make_workspace();
        // email@example.com — '@' is NOT preceded by whitespace
        let input = "contact email@example.com please";
        let result = resolve_at_refs(input, dir.path());
        assert_eq!(
            result, input,
            "email address should not trigger @-ref resolution"
        );
    }
}
