//! Workspace detection: current directory, project type, AGENT.md loading, and Git context.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

/// Type of project detected from marker files in the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Rust,   // Cargo.toml
    Node,   // package.json
    Python, // pyproject.toml or requirements.txt
    Go,     // go.mod
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rust => write!(f, "Rust"),
            Self::Node => write!(f, "Node"),
            Self::Python => write!(f, "Python"),
            Self::Go => write!(f, "Go"),
        }
    }
}

impl ProjectType {
    /// Display with emoji for TUI header (e.g. "🦀 Rust", "🐍 Python").
    pub fn display_with_emoji(&self) -> &'static str {
        match self {
            Self::Rust => "🦀 Rust",
            Self::Node => "📦 Node",
            Self::Python => "🐍 Python",
            Self::Go => "🐹 Go",
        }
    }
}

/// Git context: branch and status for injection into the system prompt.
#[derive(Debug, Clone)]
pub struct GitContext {
    /// Current branch name (None if detached or repo empty).
    pub branch: Option<String>,
    /// Output of `git status --short`, truncated to avoid token bloat.
    pub status: String,
}

impl GitContext {
    /// Format for injection into the system prompt.
    pub fn formatted(&self) -> String {
        let mut out = String::new();
        if let Some(ref b) = self.branch {
            out.push_str("Branch: ");
            out.push_str(b);
            out.push('\n');
        }
        if !self.status.is_empty() {
            out.push_str("Status:\n");
            out.push_str(&self.status);
        } else if self.branch.is_some() {
            out.push_str("Status: (clean)\n");
        }
        out
    }
}

/// Workspace: root directory, detected project type, optional AGENTS.md/AGENT.md content, and optional Git context.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Absolute path of the current working directory (at startup).
    pub root: PathBuf,
    /// Detected project type (Rust, Node, Python, Go).
    pub project_type: Option<ProjectType>,
    /// Content of AGENTS.md or AGENT.md if present (AGENTS.md takes precedence).
    pub agent_md: Option<String>,
    /// Git context (branch, status) when in a Git repo and MY_OPEN_CLAUDE_GIT_CONTEXT is enabled.
    pub git_context: Option<GitContext>,
}

/// Default max lines for git status output.
const GIT_STATUS_MAX_LINES_DEFAULT: usize = 50;
/// Default max bytes for git status output.
const GIT_STATUS_MAX_BYTES_DEFAULT: usize = 2048;

/// Configuration for Git context injection, loaded from environment variables.
#[derive(Debug, Clone)]
pub struct GitContextConfig {
    /// Whether Git context injection is enabled.
    pub enabled: bool,
    /// Max lines for git status output.
    pub max_lines: usize,
    /// Max bytes for git status output.
    pub max_bytes: usize,
}

impl GitContextConfig {
    /// Load configuration from environment variables.
    ///
    /// - `MY_OPEN_CLAUDE_GIT_CONTEXT`: 0 or false to disable; default enabled
    /// - `MY_OPEN_CLAUDE_GIT_STATUS_MAX_LINES`: default 50
    /// - `MY_OPEN_CLAUDE_GIT_STATUS_MAX_BYTES`: default 2048
    pub fn from_env() -> Self {
        let enabled = !env::var("MY_OPEN_CLAUDE_GIT_CONTEXT")
            .map(|s| s == "0" || s.eq_ignore_ascii_case("false"))
            .unwrap_or(false);

        let max_lines = env::var("MY_OPEN_CLAUDE_GIT_STATUS_MAX_LINES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(GIT_STATUS_MAX_LINES_DEFAULT);

        let max_bytes = env::var("MY_OPEN_CLAUDE_GIT_STATUS_MAX_BYTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(GIT_STATUS_MAX_BYTES_DEFAULT);

        Self {
            enabled,
            max_lines,
            max_bytes,
        }
    }
}

/// Errors that can occur when gathering Git context.
#[derive(Debug, Error)]
pub enum GitContextError {
    #[error("not a Git repository")]
    NotARepository,

    #[error("Git command failed: {0}")]
    CommandFailed(String),
}

/// Marker files for project type detection (checked in this order).
const MARKERS: &[(ProjectType, &str)] = &[
    (ProjectType::Rust, "Cargo.toml"),
    (ProjectType::Node, "package.json"),
    (ProjectType::Python, "pyproject.toml"),
    (ProjectType::Python, "requirements.txt"),
    (ProjectType::Go, "go.mod"),
];

/// Detect workspace from the current working directory.
///
/// - Resolves root to an absolute path (canonicalize when possible)
/// - Detects project type from marker files (first match wins)
/// - Loads AGENTS.md or AGENT.md if present (AGENTS.md takes precedence)
pub fn detect() -> Workspace {
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let root = root.canonicalize().unwrap_or_else(|_| root.clone());

    let project_type = detect_project_type(&root);
    let agent_md = load_agent_md(&root);
    let git_context = gather_git_context(&root);

    Workspace {
        root,
        project_type,
        agent_md,
        git_context,
    }
}

fn gather_git_context(root: &Path) -> Option<GitContext> {
    let config = GitContextConfig::from_env();
    if !config.enabled {
        log::debug!("Git context disabled by MY_OPEN_CLAUDE_GIT_CONTEXT");
        return None;
    }

    // Check if root is inside a Git repo.
    match Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(root)
        .output()
    {
        Ok(o) if o.status.success() => {}
        Ok(_) => {
            log::debug!("Git context skipped: {}", GitContextError::NotARepository);
            return None;
        }
        Err(e) => {
            log::warn!(
                "Git context skipped: {}",
                GitContextError::CommandFailed(e.to_string())
            );
            return None;
        }
    }

    let branch = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        });

    let status_out = match Command::new("git")
        .args(["status", "--short"])
        .current_dir(root)
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Ok(o) => {
            log::debug!(
                "Git context: {}",
                GitContextError::CommandFailed(format!("status exited with {:?}", o.status.code()))
            );
            return None;
        }
        Err(e) => {
            log::warn!(
                "Git context: {}",
                GitContextError::CommandFailed(e.to_string())
            );
            return None;
        }
    };

    let status = truncate_status(&status_out, config.max_lines, config.max_bytes);
    if status_out.lines().count() > config.max_lines || status_out.len() > config.max_bytes {
        log::debug!(
            "Git status truncated (max_lines={}, max_bytes={})",
            config.max_lines,
            config.max_bytes
        );
    }

    Some(GitContext { branch, status })
}

pub(crate) fn truncate_status(s: &str, max_lines: usize, max_bytes: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    let mut out = String::new();
    let mut bytes = 0usize;
    for (i, line) in lines.iter().enumerate() {
        if i >= max_lines || bytes + line.len() + 1 > max_bytes {
            if i > 0 {
                out.push_str("\n... (truncated)");
            }
            break;
        }
        if i > 0 {
            out.push('\n');
            bytes += 1;
        }
        out.push_str(line);
        bytes += line.len();
    }
    out
}

fn detect_project_type(root: &Path) -> Option<ProjectType> {
    for (pt, marker) in MARKERS {
        if root.join(marker).exists() {
            return Some(*pt);
        }
    }
    None
}

fn load_agent_md(root: &Path) -> Option<String> {
    // AGENTS.md (OpenCode/init convention) takes precedence over AGENT.md. Case-insensitive for Linux.
    let entries = std::fs::read_dir(root).ok()?;
    let mut agents_content = None;
    let mut agent_content = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
        {
            if name.eq_ignore_ascii_case("AGENTS.md") {
                agents_content = std::fs::read_to_string(&path).ok();
            } else if name.eq_ignore_ascii_case("AGENT.md") {
                agent_content = std::fs::read_to_string(&path).ok();
            }
        }
    }
    agents_content.or(agent_content)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_status_adds_truncated_marker_when_over_limit() {
        let long_status = (0..100)
            .map(|i| format!("M file_{}.txt", i))
            .collect::<Vec<_>>()
            .join("\n");

        let truncated = truncate_status(&long_status, 50, 2048);
        assert!(
            truncated.contains("(truncated)"),
            "expected truncated marker in output"
        );
        assert!(
            truncated.lines().count() <= 51,
            "expected at most 50 lines plus truncation marker"
        );
    }

    #[test]
    fn truncate_status_respects_max_lines() {
        let long_status = (0..100)
            .map(|i| format!("M file_{}.txt", i))
            .collect::<Vec<_>>()
            .join("\n");

        let truncated = truncate_status(&long_status, 10, 10000);
        let line_count = truncated.lines().count();
        assert!(
            line_count <= 11,
            "expected at most 10 lines plus optional truncation marker, got {}",
            line_count
        );
    }

    #[test]
    fn git_context_formatted_includes_branch_and_status() {
        let context = GitContext {
            branch: Some("feature/test".to_string()),
            status: "M src/main.rs\n?? new_file.txt".to_string(),
        };

        let formatted = context.formatted();
        assert!(formatted.contains("Branch: feature/test"));
        assert!(formatted.contains("M src/main.rs"));
        assert!(formatted.contains("?? new_file.txt"));
    }

    #[test]
    fn git_context_formatted_clean_status_when_empty() {
        let context = GitContext {
            branch: Some("main".to_string()),
            status: String::new(),
        };

        let formatted = context.formatted();
        assert!(formatted.contains("Branch: main"));
        assert!(formatted.contains("Status: (clean)"));
    }
}
