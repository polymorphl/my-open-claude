//! Workspace detection: current directory, project type, and AGENT.md loading.

use std::path::{Path, PathBuf};

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

/// Workspace: root directory, detected project type, and optional AGENTS.md/AGENT.md content.
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Absolute path of the current working directory (at startup).
    pub root: PathBuf,
    /// Detected project type (Rust, Node, Python, Go).
    pub project_type: Option<ProjectType>,
    /// Content of AGENTS.md or AGENT.md if present (AGENTS.md takes precedence).
    pub agent_md: Option<String>,
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

    Workspace {
        root,
        project_type,
        agent_md,
    }
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
    // AGENTS.md (OpenCode/init convention) takes precedence over AGENT.md
    for name in ["AGENTS.md", "AGENT.md"] {
        let path = root.join(name);
        if path.is_file()
            && let Ok(content) = std::fs::read_to_string(&path)
        {
            return Some(content);
        }
    }
    None
}
