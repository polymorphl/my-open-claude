//! Chat history: message list with blocks, separators, and scrollbar.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use super::super::app::{App, ChatMessage};
use super::super::constants::ACCENT_SECONDARY;
use super::super::text::{parse_markdown_inline, wrap_message};

/// Repeat a character to fill width (approximate; chars may have different display widths).
fn repeat_char(c: char, n: usize) -> String {
    std::iter::repeat_n(c, n).collect()
}

/// Add a User or Assistant message block with borders and separator.
/// Returns (start_line, end_line) for this block in the lines array.
fn add_message_block(
    lines: &mut Vec<Line<'static>>,
    label: &str,
    content: &str,
    content_width: usize,
    wrap_width: usize,
    is_error: bool,
    is_user: bool,
) -> (usize, usize) {
    let border_color = if is_user {
        Color::DarkGray
    } else {
        ACCENT_SECONDARY
    };
    let border_style = Style::default().fg(border_color);

    let start = lines.len();

    // Top border: "┌─ Label ───...──┐"
    let top_label = format!("┌─ {} ", label);
    let top_trail_len = wrap_width.saturating_sub(top_label.chars().count() + 1);
    let top_line = format!("{}{}┐", top_label, repeat_char('─', top_trail_len.max(0)));
    lines.push(Line::from(Span::styled(top_line, border_style)));

    // Content lines with left border
    for chunk in wrap_message(content, content_width) {
        let (prefix, chunk_style) = if chunk.is_empty() {
            ("  ", Style::default())
        } else if is_error {
            ("  ", Style::default().fg(Color::Red))
        } else {
            ("  ", Style::default())
        };
        let mut spans = vec![
            Span::styled("│ ", border_style),
            Span::styled(prefix, Style::default()),
        ];
        if is_error {
            spans.push(Span::styled(chunk, chunk_style));
        } else {
            spans.extend(parse_markdown_inline(&chunk));
        }
        lines.push(Line::from(spans));
    }

    // Bottom border
    let bottom_line = format!("└{}┘", repeat_char('─', wrap_width.saturating_sub(2)));
    lines.push(Line::from(Span::styled(bottom_line, border_style)));

    // Separator between messages
    let sep_line = repeat_char('─', wrap_width);
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

    for (msg_idx, msg) in app.messages.iter().enumerate() {
        match msg {
            ChatMessage::User(s) => {
                let (start, end) =
                    add_message_block(&mut lines, "You", s, content_width, wrap_width, false, true);
                message_line_ranges.push((msg_idx, start, end));
            }
            ChatMessage::Assistant(s) => {
                let is_error = s.starts_with("Error:");
                let (start, end) = add_message_block(
                    &mut lines,
                    "Assistant",
                    s,
                    content_width,
                    wrap_width,
                    is_error,
                    false,
                );
                message_line_ranges.push((msg_idx, start, end));
            }
            ChatMessage::ToolLog(s) => {
                lines.push(Line::from(Span::styled(
                    format!("  {} ", s),
                    Style::default().fg(Color::DarkGray),
                )));
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
        .track_symbol(Some("│"));
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}
