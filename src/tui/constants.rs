//! TUI constants: colors and suggestion labels.

use ratatui::style::Color;

/// Accent green color (#98FB98).
pub(super) const ACCENT: Color = Color::Rgb(152, 251, 152);

/// Actions below input: Ask (explanation), Build (writing / files, bash, etc.).
pub(super) const SUGGESTIONS: &[&str] = &["Ask", "Build"];
