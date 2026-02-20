//! Selection and copy logic for the chat history: hit-testing, extraction, and clipboard.

use std::time::{Duration, Instant};

use ratatui::layout::Position;

use super::super::app::{App, CopyTarget};

/// Message index at the current scroll position (for Cmd+C when no hover).
pub(crate) fn message_idx_at_scroll_line(app: &App) -> Option<usize> {
    let line = app.scroll_line();
    app.message_line_ranges
        .iter()
        .find(|(_, start, end)| *start <= line && line < *end)
        .map(|(idx, _, _)| *idx)
}

/// Strip leading border characters (│ , │ │ ) from a line for clean copy.
/// Preserves content indentation (e.g. leading spaces in code).
fn strip_border_prefix(line: &str) -> &str {
    let mut rest = line;
    // Strip "│ " (box-drawing vertical + space) repeatedly for nested blocks
    while let Some(remainder) = rest.strip_prefix('\u{2502}') {
        rest = remainder.strip_prefix(' ').unwrap_or(remainder);
    }
    rest
}

/// Extract selected text from rendered_lines. Returns None if selection is empty or invalid.
/// Strips border prefixes (│ , etc.) from each line for clean copy.
fn extract_selection(app: &App) -> Option<String> {
    let (sl, sc, el, ec) = app.selection?;
    let lines = &app.rendered_lines;
    if lines.is_empty() || sl >= lines.len() {
        return None;
    }
    let el = el.min(lines.len().saturating_sub(1));
    let mut parts = Vec::new();
    for (i, s) in lines.iter().enumerate() {
        if i < sl || i > el {
            continue;
        }
        let len = s.chars().count();
        let (start, end) = if sl == el {
            (sc.min(ec).min(len), sc.max(ec).min(len))
        } else if i == sl {
            (sc.min(len), len)
        } else if i == el {
            (0, ec.min(len))
        } else {
            (0, len)
        };
        if start < end {
            let segment: String = s.chars().skip(start).take(end - start).collect();
            let cleaned = strip_border_prefix(&segment);
            parts.push(cleaned.to_string());
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.join("\n"))
}

/// Copy message at msg_idx to clipboard. Returns true if successful.
pub(crate) fn try_copy_message(app: &mut App, msg_idx: usize) -> bool {
    let content = match app.messages.get(msg_idx) {
        Some(super::super::app::ChatMessage::User(s))
        | Some(super::super::app::ChatMessage::Assistant(s)) => s.clone(),
        _ => return false,
    };
    if arboard::Clipboard::new()
        .and_then(|mut c| c.set_text(content))
        .is_ok()
    {
        app.copy_toast_until = Some(Instant::now() + Duration::from_secs(2));
        true
    } else {
        false
    }
}

/// Copy selection to clipboard. Returns true if successful.
pub(crate) fn try_copy_selection(app: &mut App) -> bool {
    if let Some(content) = extract_selection(app)
        && !content.is_empty()
        && arboard::Clipboard::new()
            .and_then(|mut c| c.set_text(content))
            .is_ok()
    {
        app.copy_toast_until = Some(Instant::now() + Duration::from_secs(2));
        true
    } else {
        false
    }
}

/// Check if position is over a copyable message block; return Some(msg_idx) if so.
pub(crate) fn hit_test_message(app: &App, pos: Position) -> Option<usize> {
    let history_rect = app.history_area_rect?;
    if !history_rect.contains(pos) {
        return None;
    }
    let rel_row = pos.y.saturating_sub(history_rect.y) as usize;
    let scroll_pos = app.scroll_line();
    let clicked_line = scroll_pos + rel_row;
    app.message_line_ranges
        .iter()
        .find_map(|&(msg_idx, start, end)| {
            if start <= clicked_line && clicked_line < end {
                Some(msg_idx)
            } else {
                None
            }
        })
}

/// Map mouse position to (buffer_line, buffer_col) when inside history area.
pub(crate) fn pos_to_buffer_coords(app: &App, pos: Position) -> Option<(usize, usize)> {
    let history_rect = app.history_area_rect?;
    if !history_rect.contains(pos) {
        return None;
    }
    let rel_row = pos.y.saturating_sub(history_rect.y) as usize;
    let rel_col = pos.x.saturating_sub(history_rect.x) as usize;
    let buffer_line = app.scroll_line() + rel_row;
    Some((buffer_line, rel_col))
}

/// Hit-test for copy-on-click. Returns CopyTarget (code blocks first, then message fallback).
pub(crate) fn hit_test_copy_region(app: &App, pos: Position) -> Option<CopyTarget> {
    let history_rect = app.history_area_rect?;
    if !history_rect.contains(pos) {
        return None;
    }
    let rel_row = pos.y.saturating_sub(history_rect.y) as usize;
    let scroll_pos = app.scroll_line();
    let clicked_line = scroll_pos + rel_row;
    app.copy_regions.iter().find_map(|(start, end, target)| {
        if *start <= clicked_line && clicked_line < *end {
            Some(target.clone())
        } else {
            None
        }
    })
}
