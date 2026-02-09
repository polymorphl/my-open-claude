//! TUI constants: colors and suggestion labels.

use ratatui::style::Color;

/// Accent green color (#98FB98).
pub(super) const ACCENT: Color = Color::Rgb(152, 251, 152);

/// Actions below input: Ask (explanation), Build (writing / files, bash, etc.).
pub(super) const SUGGESTIONS: &[&str] = &["Ask", "Build"];

/// Minimalist logo when idle (single character).
pub(super) const LOGO_IDLE: &str = "◆";

/// Spinner frames for "thinking" animation (braille pattern, 4 frames).
pub(super) const LOGO_THINKING: &[&str] = &["⠋", "⠙", "⠹", "⠸"];
