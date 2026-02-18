//! Application identity from Cargo.toml.
//!
//! Single source of truth for the app name, version, and vendor used across the codebase.

/// Application name (from Cargo.toml `package.name`).
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Application version (from Cargo.toml `package.version`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Vendor / organization (paths, GitHub repo owner). Used in ProjectDirs and self-update.
pub const VENDOR: &str = "polymorphl";
