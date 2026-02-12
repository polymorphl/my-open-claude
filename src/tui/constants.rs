//! TUI constants: colors, timing, and suggestion labels.

use ratatui::style::Color;

/// Accent green color (#98FB98).
pub(super) const ACCENT: Color = Color::Rgb(152, 251, 152);

/// Secondary accent — soft cyan (#7EC8E3) that pairs well with the green.
pub(super) const ACCENT_SECONDARY: Color = Color::Rgb(126, 200, 227);

/// Actions below input: Ask (explanation), Build (writing / files, bash, etc.).
pub(super) const SUGGESTIONS: &[&str] = &["Ask", "Build"];

/// Event poll timeout in milliseconds (main loop).
pub(crate) const EVENT_POLL_TIMEOUT_MS: u64 = 100;

/// Max length for conversation title preview (with ellipsis when truncated).
pub(crate) const TITLE_PREVIEW_MAX_LEN: usize = 60;

/// Scroll amount for arrow keys and mouse wheel.
pub(crate) const SCROLL_LINES_SMALL: usize = 3;

/// Scroll amount for PageUp/PageDown.
pub(crate) const SCROLL_LINES_PAGE: usize = 10;

/// Input textarea height (number of visible lines; -2 for block borders = inner lines).
pub(crate) const INPUT_LINES: u16 = 7;

/// Minimalist logo when idle (single character).
pub(super) const LOGO_IDLE: &str = "◆";

/// Spinner frames for "thinking" animation (braille pattern, 4 frames).
pub(super) const LOGO_THINKING: &[&str] = &["⠋", "⠙", "⠹", "⠸"];
