//! TUI rendering: layout and widgets for the chat interface.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState,
};
use std::env;
use std::time::Instant;

use crate::core::models::filter_models;

use super::app::{App, ChatMessage};
use super::constants::{ACCENT, LOGO_IDLE, LOGO_THINKING, SUGGESTIONS};
use super::text::{parse_markdown_inline, wrap_message};

/// Start time for header animation phase (thinking spinner).
static HEADER_START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

pub(super) fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(3),
            Constraint::Length(6),
        ])
        .split(area);

    let header_area = chunks[0];
    let history_area = chunks[1];
    let input_section = chunks[2];

    draw_header(f, app, header_area);
    draw_history(f, app, history_area);
    draw_input_section(f, app, input_section);

    if let Some(ref popup) = app.confirm_popup {
        draw_confirm_popup(f, area, &popup.command);
    }
    if let Some(ref mut selector) = app.model_selector {
        draw_model_selector_popup(f, area, selector);
    }
}

fn is_thinking(app: &App) -> bool {
    app.messages
        .last()
        .map(|m| matches!(m, ChatMessage::Thinking))
        .unwrap_or(false)
}

/// Max width for model name in header; longer names are truncated with "…".
const MODEL_HEADER_WIDTH: u16 = 28;
/// Width for credits display in header (e.g. "$12.50" or "—" when loading).
const CREDITS_HEADER_WIDTH: u16 = 12;

fn draw_header(f: &mut Frame, app: &mut App, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(MODEL_HEADER_WIDTH),
            Constraint::Length(CREDITS_HEADER_WIDTH),
        ])
        .split(area);

    let logo_area = header_chunks[0];
    let title_area = header_chunks[1];
    let model_area = header_chunks[2];
    let credits_area = header_chunks[3];

    let logo_symbol = if is_thinking(app) {
        let start = HEADER_START.get_or_init(Instant::now);
        let phase = start.elapsed().as_millis() as usize;
        let frame = (phase / 80) % LOGO_THINKING.len();
        LOGO_THINKING[frame]
    } else {
        LOGO_IDLE
    };
    let logo_line = Line::from(Span::styled(
        format!("{} ", logo_symbol),
        Style::default().fg(ACCENT),
    ));
    let logo_para = Paragraph::new(logo_line);
    f.render_widget(logo_para, logo_area);

    let title = Line::from(vec![Span::styled(
        "my-open-claude ",
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
    )]);
    let title_block = Paragraph::new(title).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(title_block, title_area);

    let max_len = MODEL_HEADER_WIDTH as usize;
    let model_display = if app.model_name.chars().count() > max_len {
        let chars: Vec<char> = app.model_name.chars().collect();
        let start = chars.len().saturating_sub(max_len.saturating_sub(1));
        format!("…{}", chars[start..].iter().collect::<String>())
    } else {
        app.model_name.clone()
    };
    let model_line = Line::from(Span::styled(
        model_display,
        Style::default().fg(Color::DarkGray),
    ));
    let model_para = Paragraph::new(model_line).alignment(ratatui::layout::Alignment::Right);
    f.render_widget(model_para, model_area);

    let credits_display = match &app.credit_data {
        Some((total, used)) => {
            let balance = (*total - *used).max(0.0);
            format!("${:.2}", balance)
        }
        None => "—".to_string(),
    };
    let credits_line = Line::from(Span::styled(
        credits_display,
        Style::default().fg(ACCENT).add_modifier(Modifier::UNDERLINED),
    ));
    let credits_para = Paragraph::new(credits_line).alignment(ratatui::layout::Alignment::Right);
    f.render_widget(credits_para, credits_area);
    app.credits_header_rect = Some(credits_area);
}

/// Draw a user or assistant message block (label + wrapped content).
fn draw_message_block(
    lines: &mut Vec<Line<'static>>,
    label: impl Into<String>,
    content: &str,
    content_width: usize,
) {
    lines.push(Line::from(vec![
        Span::styled(label.into(), Style::default().fg(Color::DarkGray)),
        Span::styled("→ ", Style::default().fg(ACCENT)),
    ]));
    for chunk in wrap_message(content, content_width) {
        if chunk.is_empty() {
            lines.push(Line::from(Span::raw("")));
        } else {
            let mut spans = vec![Span::raw("  ")];
            spans.extend(parse_markdown_inline(&chunk));
            lines.push(Line::from(spans));
        }
    }
}

fn draw_history(f: &mut Frame, app: &mut App, history_area: Rect) {
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
                draw_message_block(&mut lines, "You ", s, content_width);
            }
            ChatMessage::Assistant(s) => {
                draw_message_block(&mut lines, "Assistant ", s, content_width);
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
    let max_scroll = total_lines.saturating_sub(visible.min(1));
    app.last_max_scroll = max_scroll;
    let scroll_pos = app.scroll_line().min(max_scroll);
    let start = scroll_pos;
    let end = (start + visible).min(total_lines);
    let visible_lines: Vec<Line> = lines.into_iter().skip(start).take(end - start).collect();

    let paragraph = Paragraph::new(visible_lines);
    f.render_widget(paragraph, text_area);

    let mut scrollbar_state = ScrollbarState::default()
        .position(scroll_pos)
        .content_length(total_lines);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_symbol("█")
        .track_symbol(Some("│"));
    f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
}

fn draw_input_section(f: &mut Frame, app: &mut App, input_section: Rect) {
    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(input_section);

    let input_area = input_chunks[0];
    let suggestions_area = input_chunks[1];
    let shortcuts_area = input_chunks[2];

    let input_content = if app.input.is_empty() {
        Span::styled("Ask anything... ", Style::default().fg(Color::DarkGray))
    } else {
        Span::raw(app.input.as_str())
    };
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = input_block.inner(input_area);
    let input_paragraph = Paragraph::new(Line::from(input_content))
        .block(input_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(input_paragraph, input_area);
    let cx = inner.x + app.input.len().min(inner.width as usize) as u16;
    let cy = input_area.y + 1;
    f.set_cursor_position(Position::new(cx, cy));

    let suggestion_spans: Vec<Span> = SUGGESTIONS
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let selected = i == app.selected_suggestion;
            Span::styled(
                format!(" {} ", s),
                if selected {
                    Style::default().fg(Color::Black).bg(ACCENT)
                } else {
                    Style::default().fg(Color::DarkGray)
                },
            )
        })
        .collect();
    let suggestions_line = Line::from(suggestion_spans);
    let suggestions_para =
        Paragraph::new(suggestions_line).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(suggestions_para, suggestions_area);

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(72)])
        .split(shortcuts_area);
    let path_area = bottom_chunks[0];
    let shortcuts_area_right = bottom_chunks[1];

    let path_display = env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .unwrap_or_else(|| "?".to_string());
    let max_path_len = path_area.width as usize;
    let path_display = if path_display.chars().count() > max_path_len && max_path_len > 2 {
        let tail: String = path_display.chars().rev().take(max_path_len - 1).collect();
        format!("…{}", tail.chars().rev().collect::<String>())
    } else {
        path_display
    };
    let path_line = Line::from(Span::styled(
        path_display,
        Style::default().fg(Color::DarkGray),
    ));
    let path_para = Paragraph::new(path_line).alignment(ratatui::layout::Alignment::Left);
    f.render_widget(path_para, path_area);

    let shortcuts = Line::from(vec![
        Span::styled("Enter ", Style::default().fg(Color::DarkGray)),
        Span::raw("send"),
        Span::styled("  ↑↓ ", Style::default().fg(Color::DarkGray)),
        Span::raw("scroll"),
        Span::styled("  Alt+M/F2 ", Style::default().fg(Color::DarkGray)),
        Span::raw("model"),
        Span::styled("  credits ", Style::default().fg(Color::DarkGray)),
        Span::raw("(click) "),
        Span::styled("  Ctrl+C ", Style::default().fg(Color::DarkGray)),
        Span::raw("quit"),
    ]);
    let shortcuts_para = Paragraph::new(shortcuts).alignment(ratatui::layout::Alignment::Right);
    f.render_widget(shortcuts_para, shortcuts_area_right);
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let vertical_areas = vertical.split(area);
    let horizontal_areas = horizontal.split(vertical_areas[0]);
    horizontal_areas[0]
}

fn draw_confirm_popup(f: &mut Frame, area: Rect, command: &str) {
    let popup_rect = popup_area(area, 70, 25);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(" ⚠ Destructive command ");

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("Command: "),
            Span::styled(
                command,
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("y ", Style::default().fg(ACCENT)),
            Span::raw("confirm  "),
            Span::styled("n ", Style::default().fg(Color::DarkGray)),
            Span::raw("cancel"),
        ]),
    ];
    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(Clear, popup_rect);
    f.render_widget(paragraph, popup_rect);
}

fn draw_model_selector_popup(
    f: &mut Frame,
    area: Rect,
    selector: &mut super::app::ModelSelectorState,
) {
    let popup_rect = popup_area(area, 60, 50);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(" Select model (Alt+M / F2) ");

    let inner = block.inner(popup_rect);
    f.render_widget(Clear, popup_rect);
    f.render_widget(block, popup_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(inner);
    let filter_area = chunks[0];
    let list_area = chunks[1];
    let hint_area = chunks[2];

    // Filter input
    let filter_content = if selector.filter.is_empty() {
        Span::styled("Filter... ", Style::default().fg(Color::DarkGray))
    } else {
        Span::raw(selector.filter.as_str())
    };
    let filter_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let filter_inner = filter_block.inner(filter_area);
    let filter_para = Paragraph::new(Line::from(filter_content))
        .block(filter_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(filter_para, filter_area);
    let cx = filter_inner.x + selector.filter.chars().count().min(filter_inner.width as usize) as u16;
    let cy = filter_area.y + 1;
    f.set_cursor_position(Position::new(cx, cy));

    if let Some(ref err) = selector.fetch_error {
        let para = Paragraph::new(Line::from(Span::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        )));
        f.render_widget(para, list_area);
    } else if selector.models.is_empty() {
        let para = Paragraph::new(Line::from(Span::styled(
            "Loading...",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )));
        f.render_widget(para, list_area);
    } else {
        let filtered = filter_models(&selector.models, &selector.filter);
        let clamped_index = selector
            .selected_index
            .min(filtered.len().saturating_sub(1));
        selector.selected_index = clamped_index;

        if filtered.is_empty() {
            let msg = if selector.filter.is_empty() {
                "No models"
            } else {
                "No models match filter"
            };
            let para = Paragraph::new(Line::from(Span::styled(
                msg,
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            )));
            f.render_widget(para, list_area);
        } else {
            let items: Vec<ListItem> = filtered
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let style = if i == selector.selected_index {
                        Style::default().fg(Color::Black).bg(ACCENT)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!(" {} ", m.name)).style(style)
                })
                .collect();

            selector.list_state.select(Some(selector.selected_index));

            let list = List::new(items).highlight_style(Style::default().fg(Color::Black).bg(ACCENT));
            f.render_stateful_widget(list, list_area, &mut selector.list_state);
        }
    }

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("↑↓ ", Style::default().fg(Color::DarkGray)),
        Span::raw("select  "),
        Span::styled("Enter ", Style::default().fg(Color::DarkGray)),
        Span::raw("confirm  "),
        Span::styled("Esc ", Style::default().fg(Color::DarkGray)),
        Span::raw("cancel  "),
        Span::styled("type ", Style::default().fg(Color::DarkGray)),
        Span::raw("filter  "),
        Span::styled("Alt+M/F2 ", Style::default().fg(Color::DarkGray)),
        Span::raw("reopen"),
    ]));
    f.render_widget(hint, hint_area);
}
