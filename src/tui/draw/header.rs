//! Header: logo, conversation count, title, model name, credits.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use std::time::Instant;

use crate::core::history;

use super::super::app::App;

/// Start time for header animation phase (thinking spinner).
pub(crate) static HEADER_START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Max width for model name in header; longer names are truncated with "…".
const MODEL_HEADER_WIDTH: u16 = 28;
/// Width for credits display in header (e.g. "$12.50" or "—" when loading).
const CREDITS_HEADER_WIDTH: u16 = 12;

/// Title text for header (used for centering). Append " *" when dirty.
pub(crate) fn title_text(app: &App) -> String {
    if app.is_dirty() {
        "my-open-claude * ".to_string()
    } else {
        "my-open-claude ".to_string()
    }
}

pub(crate) fn is_thinking(app: &App) -> bool {
    app.messages
        .last()
        .map(|m| matches!(m, super::super::app::ChatMessage::Thinking))
        .unwrap_or(false)
}

pub(crate) fn draw_header(f: &mut Frame, app: &mut App, area: Rect, accent: Color) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(8),
            Constraint::Min(0),
            Constraint::Length(MODEL_HEADER_WIDTH),
            Constraint::Length(CREDITS_HEADER_WIDTH),
        ])
        .split(area);

    let logo_area = header_chunks[0];
    let model_area = header_chunks[2];
    let credits_area = header_chunks[3];

    let logo_symbol = if is_thinking(app) {
        let start = HEADER_START.get_or_init(Instant::now);
        let phase = start.elapsed().as_millis() as usize;
        let frame = (phase / 80) % super::super::constants::LOGO_THINKING.len();
        super::super::constants::LOGO_THINKING[frame]
    } else {
        super::super::constants::LOGO_IDLE
    };
    let count = history::list_conversations().len();
    let logo_line = Line::from(vec![
        Span::styled(
            format!("{} ", logo_symbol),
            Style::default().fg(accent),
        ),
        Span::styled(
            format!("{} ", count),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    f.render_widget(Paragraph::new(logo_line), logo_area);

    let title_str = title_text(app);
    let title_len = title_str.len() as u16;
    let title_area = Rect {
        x: area.x + area.width.saturating_sub(title_len) / 2,
        y: area.y,
        width: title_len.min(area.width),
        height: area.height,
    };
    let title = Line::from(vec![Span::styled(
        title_str,
        Style::default().fg(accent).add_modifier(Modifier::BOLD),
    )]);
    f.render_widget(Paragraph::new(title), title_area);

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
    f.render_widget(
        Paragraph::new(model_line).alignment(ratatui::layout::Alignment::Right),
        model_area,
    );

    let credits_display = match &app.credit_data {
        Some((total, used)) => {
            let balance = (*total - *used).max(0.0);
            format!("${:.2}", balance)
        }
        None => "—".to_string(),
    };
    let credits_line = Line::from(Span::styled(
        credits_display,
        Style::default().fg(accent).add_modifier(Modifier::UNDERLINED),
    ));
    f.render_widget(
        Paragraph::new(credits_line).alignment(ratatui::layout::Alignment::Right),
        credits_area,
    );
    app.credits_header_rect = Some(credits_area);
}
