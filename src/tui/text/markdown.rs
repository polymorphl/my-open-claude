//! Inline Markdown parsing: **bold**, `code`, headings, lists, tables, links.

use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use crate::tui::constants::ACCENT;

/// Parse inline Markdown: **bold**, `code`, headings, bullet/numbered lists, [links](url).
pub(crate) fn parse_markdown_inline(s: &str) -> Vec<Span<'static>> {
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
    // Bullet list: - or * at line start
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
        spans.push(Span::styled("• ", Style::default().fg(ACCENT)));
        spans.extend(parse_markdown_inline_inner(trimmed.get(2..).unwrap_or("")));
        return spans;
    }
    // Table row: | cell1 | cell2 |
    if trimmed.starts_with('|') && trimmed.contains('|') {
        let cells: Vec<&str> = trimmed
            .split('|')
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .collect();
        if !cells.is_empty() {
            let mut first = true;
            for cell in cells {
                if !first {
                    spans.push(Span::styled(" │ ", Style::default().fg(ACCENT)));
                }
                spans.extend(parse_markdown_inline_inner(cell));
                first = false;
            }
            return spans;
        }
    }
    // Numbered list: 1. 2. etc. at line start
    if let Some((num, rest_after)) = parse_numbered_list_prefix(trimmed) {
        spans.push(Span::styled(
            format!("{} ", num),
            Style::default().fg(ACCENT),
        ));
        spans.extend(parse_markdown_inline_inner(rest_after));
        return spans;
    }
    spans.extend(parse_markdown_inline_inner(s));
    spans
}

/// Parse "N. " or "N) " at start. Returns (number, rest) or None.
fn parse_numbered_list_prefix(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    let mut digits = 0;
    for c in s.chars() {
        if c.is_ascii_digit() {
            digits += 1;
        } else {
            break;
        }
    }
    if digits == 0 {
        return None;
    }
    let num = &s[..digits];
    let rest = &s[digits..];
    if rest.starts_with(". ") || rest.starts_with(") ") {
        Some((num, &rest[2..]))
    } else {
        None
    }
}

/// Parse **bold**, `code`, [text](url) in the rest of a line.
fn parse_markdown_inline_inner(s: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut rest = s;
    while !rest.is_empty() {
        let next_bold = rest.find("**");
        let next_code = rest.find('`');
        let next_link = rest.find('[');
        let (which, pos) = match (next_bold, next_code, next_link) {
            (Some(b), None, None) => (0, b),
            (None, Some(c), None) => (1, c),
            (None, None, Some(l)) => (2, l),
            (Some(b), Some(c), None) => (if b <= c { 0 } else { 1 }, b.min(c)),
            (Some(b), None, Some(l)) => (if b <= l { 0 } else { 2 }, b.min(l)),
            (None, Some(c), Some(l)) => (if c <= l { 1 } else { 2 }, c.min(l)),
            (Some(b), Some(c), Some(l)) => {
                let p = b.min(c).min(l);
                let which = if p == b {
                    0
                } else if p == c {
                    1
                } else {
                    2
                };
                (which, p)
            }
            (None, None, None) => {
                spans.push(Span::raw(rest.to_string()));
                break;
            }
        };
        if pos > 0 {
            spans.push(Span::raw(rest[..pos].to_string()));
        }
        rest = &rest[pos..];
        if which == 0 && rest.starts_with("**") {
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
        } else if which == 2 && rest.starts_with('[') {
            rest = &rest[1..];
            if let Some(end_br) = rest.find(']') {
                let text = &rest[..end_br];
                rest = &rest[end_br + 1..];
                if rest.starts_with('(') {
                    rest = &rest[1..];
                    if let Some(end_paren) = rest.find(')') {
                        let _url = &rest[..end_paren];
                        rest = &rest[end_paren + 1..];
                        spans.push(Span::styled(
                            text.to_string(),
                            Style::default()
                                .fg(ACCENT)
                                .add_modifier(Modifier::UNDERLINED),
                        ));
                    } else {
                        spans.push(Span::raw(format!("[{}]", text)));
                    }
                } else {
                    spans.push(Span::raw(format!("[{}]", text)));
                }
            } else {
                spans.push(Span::raw("[".to_string()));
            }
        } else if which == 1 && rest.starts_with('`') {
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
