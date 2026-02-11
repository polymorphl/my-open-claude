//! Text utilities: markdown parsing and line wrapping for the chat display.

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use super::constants::ACCENT;

/// Parse inline Markdown: **bold**, `code`, and headings (# / ## / ###) at line start.
pub(super) fn parse_markdown_inline(s: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let trimmed = s.trim_start();
    // Heading: starts with one or more #
    if trimmed.starts_with('#') {
        let content = trimmed.trim_start_matches('#').trim_start();
        if !content.is_empty() {
            spans.push(Span::styled(
                content.to_string(),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ));
        }
        return spans;
    }
    // Otherwise parse ** and ` in the rest
    let mut rest = s;
    while !rest.is_empty() {
        let next_bold = rest.find("**");
        let next_code = rest.find('`');
        let (use_bold, pos) = match (next_bold, next_code) {
            (Some(b), None) => (true, b),
            (None, Some(c)) => (false, c),
            (Some(b), Some(c)) => (b <= c, b.min(c)),
            (None, None) => {
                spans.push(Span::raw(rest.to_string()));
                break;
            }
        };
        if pos > 0 {
            spans.push(Span::raw(rest[..pos].to_string()));
        }
        rest = &rest[pos..];
        if use_bold && rest.starts_with("**") {
            rest = &rest[2..];
            if let Some(end) = rest.find("**") {
                spans.push(Span::styled(
                    rest[..end].to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                rest = &rest[end + 2..];
            } else {
                spans.push(Span::raw("**".to_string()));
            }
        } else if rest.starts_with('`') {
            rest = &rest[1..];
            if let Some(end) = rest.find('`') {
                spans.push(Span::styled(
                    rest[..end].to_string(),
                    Style::default().fg(ACCENT),
                ));
                rest = &rest[end + 1..];
            } else {
                spans.push(Span::raw("`".to_string()));
            }
        }
    }
    spans
}

/// Split a message into display lines respecting message newlines, then wrap to `width`.
pub(super) fn wrap_message(msg: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in msg.split('\n') {
        if line.is_empty() {
            out.push(String::new());
        } else {
            for chunk in wrap_text(line, width) {
                out.push(chunk);
            }
        }
    }
    out
}

/// Split text into lines of max byte width `width`, only cutting at UTF-8 character boundaries.
fn wrap_text(s: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![s.to_string()];
    }
    let mut out = Vec::new();
    let mut rest = s;
    while !rest.is_empty() {
        if rest.len() <= width {
            out.push(rest.to_string());
            break;
        }
        // Last byte index that is a character boundary and <= width.
        let boundary = rest
            .char_indices()
            .rfind(|(i, _)| *i <= width)
            .map(|(i, _)| i);
        let boundary = match boundary {
            Some(b) if b > 0 => b,
            _ => {
                // First character exceeds width, cut after the first character.
                rest.char_indices()
                    .nth(1)
                    .map(|(i, _)| i)
                    .unwrap_or(rest.len())
            }
        };
        let (chunk, next) = if let Some(space) = rest[..boundary].rfind(' ') {
            let chunk = rest[..space].trim_end();
            let next = rest[space..].trim_start();
            (chunk, next)
        } else {
            (&rest[..boundary], rest[boundary..].trim_start())
        };
        out.push(chunk.to_string());
        rest = next;
    }
    out
}
