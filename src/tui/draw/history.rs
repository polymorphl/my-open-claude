//! Chat history: message list with blocks, separators, code blocks, and scrollbar.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use super::super::app::{App, ChatMessage};
use super::super::constants::{ACCENT, ACCENT_SECONDARY};
use super::super::text::{
    MessageSegment, parse_markdown_inline, parse_message_segments, wrap_message,
};

/// Repeat a character to fill width (approximate; chars may have different display widths).
fn repeat_char(c: char, n: usize) -> String {
    std::iter::repeat_n(c, n).collect()
}

const TOOL_LOG_PREFIX: &str = "→ ";

/// Parse tool log format "→ ToolName: args" into (tool_name, args) if it matches.
fn parse_tool_log(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if !s.starts_with(TOOL_LOG_PREFIX) {
        return None;
    }
    let rest = &s[TOOL_LOG_PREFIX.len()..];
    let colon_pos = rest.find(": ")?;
    let tool_name = rest[..colon_pos].trim();
    let args = rest[colon_pos + 2..].trim_start();
    if tool_name.is_empty() {
        None
    } else {
        Some((tool_name, args))
    }
}

/// Render tool log lines with structured styling: tool name highlighted, args wrapped.
fn add_tool_log_lines(lines: &mut Vec<Line<'static>>, s: &str, content_width: usize) {
    let marker_style = Style::default().fg(ACCENT).add_modifier(Modifier::BOLD);
    let tool_style = Style::default().fg(ACCENT).add_modifier(Modifier::BOLD);
    let args_style = Style::default().fg(ACCENT_SECONDARY);

    let prefix = "  ┃ ";
    let prefix_len = prefix.chars().count();

    if let Some((tool_name, args)) = parse_tool_log(s) {
        let header = format!("{}: ", tool_name);
        let header_char_len = header.chars().count();
        let available = content_width.saturating_sub(prefix_len);
        let args_width = available.saturating_sub(header_char_len);

        let mut first_line = true;
        for chunk in wrap_message(args, args_width.max(1)) {
            let mut spans = vec![Span::styled(prefix.to_string(), marker_style)];
            if first_line {
                spans.push(Span::styled(header.to_string(), tool_style));
                first_line = false;
            } else {
                spans.push(Span::styled(
                    " ".repeat(header_char_len.min(available)),
                    Style::default(),
                ));
            }
            spans.push(Span::styled(chunk, args_style));
            lines.push(Line::from(spans));
        }
        if first_line {
            lines.push(Line::from(vec![
                Span::styled(prefix.to_string(), marker_style),
                Span::styled(format!("{} ", header), tool_style),
            ]));
        }
    } else {
        for chunk in
            super::super::text::wrap_message(s, content_width.saturating_sub(prefix_len).max(1))
        {
            lines.push(Line::from(vec![
                Span::styled(prefix.to_string(), marker_style),
                Span::styled(format!("{} ", chunk), args_style),
            ]));
        }
    }
}

/// Parameters for rendering a message block.
struct MessageBlockParams<'a> {
    label: &'a str,
    content: &'a str,
    content_width: usize,
    wrap_width: usize,
    is_error: bool,
    is_user: bool,
    stream_cursor: bool,
    /// Unix timestamp (seconds) when message was created; None for loaded history.
    timestamp: Option<u64>,
}

/// Add a User or Assistant message block with borders, code blocks, and separator.
/// Returns (start_line, end_line) for this block in the lines array.
fn add_message_block(lines: &mut Vec<Line<'static>>, p: MessageBlockParams<'_>) -> (usize, usize) {
    let border_color = if p.is_user {
        Color::DarkGray
    } else {
        ACCENT_SECONDARY
    };
    let border_style = Style::default().fg(border_color);
    let code_inner_width = p.content_width.saturating_sub(2);

    let start = lines.len();

    // Top border: "┌─ Label ───...──┐" or "┌─ Label 14:32 ───...──┐"
    let time_suffix = p
        .timestamp
        .map(|unix_secs| {
            let hour = (unix_secs % 86400) / 3600;
            let min = (unix_secs % 3600) / 60;
            format!(" {:02}:{:02}", hour, min)
        })
        .unwrap_or_default();
    let top_label = if time_suffix.is_empty() {
        format!("┌─ {} ", p.label)
    } else {
        format!("┌─ {} {} ", p.label, time_suffix.trim())
    };
    let top_trail_len = p.wrap_width.saturating_sub(top_label.chars().count() + 1);
    let top_line = format!("{}{}┐", top_label, repeat_char('─', top_trail_len.max(0)));
    lines.push(Line::from(Span::styled(top_line, border_style)));

    let segments = parse_message_segments(p.content);

    for segment in &segments {
        match segment {
            MessageSegment::Text(text) => {
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    continue;
                }
                for chunk in wrap_message(trimmed, p.content_width) {
                    let (prefix, chunk_style) = if chunk.is_empty() {
                        ("  ", Style::default())
                    } else if p.is_error {
                        ("  ", Style::default().fg(Color::Red))
                    } else {
                        ("  ", Style::default())
                    };
                    let mut spans = vec![
                        Span::styled("│ ", border_style),
                        Span::styled(prefix, Style::default()),
                    ];
                    if p.is_error {
                        spans.push(Span::styled(chunk.clone(), chunk_style));
                    } else {
                        spans.extend(parse_markdown_inline(&chunk));
                    }
                    lines.push(Line::from(spans));
                }
            }
            MessageSegment::CodeBlock { lang, code } => {
                let lang_label = if lang.is_empty() { "code" } else { lang };
                let code_header = format!("┌─ {} ", lang_label);
                let code_trail_len =
                    code_inner_width.saturating_sub(code_header.chars().count() + 1);
                let code_header_line = format!(
                    "{}{}┐",
                    code_header,
                    repeat_char('─', code_trail_len.max(0))
                );
                lines.push(Line::from(vec![
                    Span::styled("│ ", border_style),
                    Span::styled(code_header_line, Style::default().fg(ACCENT_SECONDARY)),
                ]));
                for code_line in code.split('\n') {
                    for chunk in wrap_message(code_line, code_inner_width) {
                        lines.push(Line::from(vec![
                            Span::styled("│ ", border_style),
                            Span::styled("│ ", Style::default().fg(ACCENT_SECONDARY)),
                            Span::styled(chunk, Style::default().fg(ACCENT_SECONDARY)),
                        ]));
                    }
                }
                let code_footer =
                    format!("└{}┘", repeat_char('─', code_inner_width.saturating_sub(2)));
                lines.push(Line::from(vec![
                    Span::styled("│ ", border_style),
                    Span::styled(code_footer, Style::default().fg(ACCENT_SECONDARY)),
                ]));
            }
        }
    }

    if p.stream_cursor {
        let cursor = "▌";
        lines.push(Line::from(vec![
            Span::styled("│ ", border_style),
            Span::styled(
                format!("  {} ", cursor),
                Style::default().fg(ACCENT_SECONDARY),
            ),
        ]));
    }

    // Bottom border
    let bottom_line = format!("└{}┘", repeat_char('─', p.wrap_width.saturating_sub(2)));
    lines.push(Line::from(Span::styled(bottom_line, border_style)));

    // Separator between messages
    let sep_line = repeat_char('─', p.wrap_width);
    lines.push(Line::from(Span::styled(
        sep_line,
        Style::default().fg(Color::DarkGray),
    )));

    let end = lines.len();
    (start, end)
}

pub(crate) fn draw_history(f: &mut Frame, app: &mut App, history_area: Rect) {
    let history_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(history_area);
    let text_area = history_chunks[0];
    let scrollbar_area = history_chunks[1];
    let wrap_width = text_area.width as usize;
    let content_width = wrap_width.saturating_sub(5);
    app.last_content_width = Some(content_width);
    app.history_area_rect = Some(text_area);

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut message_line_ranges: Vec<(usize, usize, usize)> = Vec::new();

    let msg_count = app.messages.len();
    for (msg_idx, msg) in app.messages.iter().enumerate() {
        let timestamp = if app.show_timestamps {
            app.message_timestamps.get(msg_idx).copied().flatten()
        } else {
            None
        };
        match msg {
            ChatMessage::User(s) => {
                let (start, end) = add_message_block(
                    &mut lines,
                    MessageBlockParams {
                        label: "You",
                        content: s,
                        content_width,
                        wrap_width,
                        is_error: false,
                        is_user: true,
                        stream_cursor: false,
                        timestamp,
                    },
                );
                message_line_ranges.push((msg_idx, start, end));
            }
            ChatMessage::Assistant(s) => {
                let is_error = s.starts_with("Error:");
                let is_last_and_streaming =
                    app.is_streaming && msg_idx == msg_count.saturating_sub(1);
                let (start, end) = add_message_block(
                    &mut lines,
                    MessageBlockParams {
                        label: "Assistant",
                        content: s,
                        content_width,
                        wrap_width,
                        is_error,
                        is_user: false,
                        stream_cursor: is_last_and_streaming,
                        timestamp,
                    },
                );
                message_line_ranges.push((msg_idx, start, end));
            }
            ChatMessage::ToolLog(s) => {
                add_tool_log_lines(&mut lines, s, content_width);
            }
            ChatMessage::Thinking => {
                lines.push(Line::from(vec![Span::styled(
                    "  Thinking... ",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                )]));
            }
        }
    }

    app.message_line_ranges = message_line_ranges;

    let total_lines = lines.len();
    let visible = text_area.height as usize;
    let max_scroll = total_lines.saturating_sub(visible.max(1));
    app.last_max_scroll = max_scroll;
    let scroll_pos = app.scroll_line().min(max_scroll);
    let start = scroll_pos;
    let end = (start + visible).min(total_lines);
    let visible_lines: Vec<Line> = lines.into_iter().skip(start).take(end - start).collect();

    f.render_widget(Paragraph::new(visible_lines), text_area);

    let mut scrollbar_state = ScrollbarState::default()
        .position(scroll_pos)
        .content_length(total_lines);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_symbol("█")
        .thumb_style(Style::default().fg(ACCENT_SECONDARY))
        .track_symbol(Some("│"));
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}
