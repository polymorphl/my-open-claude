//! Chat history: message list with scrollbar.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};

use super::super::app::{App, ChatMessage};
use super::super::constants::ACCENT;
use super::super::text::{parse_markdown_inline, wrap_message};

/// Draw a user or assistant message block (label + wrapped content).
/// When `is_error` is true, content is displayed in red.
fn draw_message_block(
    lines: &mut Vec<Line<'static>>,
    label: impl Into<String>,
    content: &str,
    content_width: usize,
    is_error: bool,
) {
    lines.push(Line::from(vec![
        Span::styled(label.into(), Style::default().fg(Color::DarkGray)),
        Span::styled("→ ", Style::default().fg(ACCENT)),
    ]));
    for chunk in wrap_message(content, content_width) {
        if chunk.is_empty() {
            lines.push(Line::from(Span::raw("")));
        } else if is_error {
            let mut spans = vec![Span::raw("  ")];
            spans.push(Span::styled(chunk, Style::default().fg(Color::Red)));
            lines.push(Line::from(spans));
        } else {
            let mut spans = vec![Span::raw("  ")];
            spans.extend(parse_markdown_inline(&chunk));
            lines.push(Line::from(spans));
        }
    }
}

pub(crate) fn draw_history(f: &mut Frame, app: &mut App, history_area: Rect) {
    let history_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(history_area);
    let text_area = history_chunks[0];
    let scrollbar_area = history_chunks[1];
    let wrap_width = text_area.width as usize;
    let content_width = wrap_width.saturating_sub(2);
    app.last_content_width = Some(content_width);

    let mut lines: Vec<Line<'static>> = Vec::new();
    for msg in &app.messages {
        match msg {
            ChatMessage::User(s) => {
                draw_message_block(&mut lines, "You ", s, content_width, false)
            }
            ChatMessage::Assistant(s) => {
                let is_error = s.starts_with("Error:");
                draw_message_block(&mut lines, "Assistant ", s, content_width, is_error)
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
