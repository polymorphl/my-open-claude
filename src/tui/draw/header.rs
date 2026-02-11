//! Header: logo, conversation count, title, model name, token usage, credits.

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
/// Width for token usage display (e.g. "12k/128k").
const TOKENS_HEADER_WIDTH: u16 = 14;
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

/// Format a token count in compact form: 1234 -> "1k", 128000 -> "128k", 1500000 -> "1.5M".
fn format_tokens_compact(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        let m = tokens as f64 / 1_000_000.0;
        if m == m.floor() {
            format!("{}M", m as u64)
        } else {
            format!("{:.1}M", m)
        }
    } else if tokens >= 1_000 {
        let k = tokens as f64 / 1_000.0;
        if k == k.floor() {
            format!("{}k", k as u64)
        } else {
            format!("{:.1}k", k)
        }
    } else {
        format!("{}", tokens)
    }
}

/// Choose color based on token usage ratio: green < 50%, yellow 50-80%, red > 80%.
fn token_usage_color(used: u64, total: u64) -> Color {
    if total == 0 {
        return Color::DarkGray;
    }
    let ratio = used as f64 / total as f64;
    if ratio > 0.80 {
        Color::Red
    } else if ratio > 0.50 {
        Color::Yellow
    } else {
        Color::Green
    }
}

pub(crate) fn draw_header(f: &mut Frame, app: &mut App, area: Rect, accent: Color) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(8),
            Constraint::Min(0),
            Constraint::Length(MODEL_HEADER_WIDTH),
            Constraint::Length(TOKENS_HEADER_WIDTH),
            Constraint::Length(CREDITS_HEADER_WIDTH),
        ])
        .split(area);

    let logo_area = header_chunks[0];
    let model_area = header_chunks[2];
    let tokens_area = header_chunks[3];
    let credits_area = header_chunks[4];

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
        Span::styled(format!("{} ", logo_symbol), Style::default().fg(accent)),
        Span::styled(format!("{} ", count), Style::default().fg(Color::DarkGray)),
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

    // Token usage display: "used/context" with color coding.
    let tokens_display = match &app.token_usage {
        Some(usage) => {
            let used = usage.total_tokens;
            let ctx = app.context_length;
            let color = token_usage_color(used, ctx);
            let text = format!(
                "{}/{}",
                format_tokens_compact(used),
                format_tokens_compact(ctx)
            );
            Line::from(Span::styled(text, Style::default().fg(color)))
        }
        None => {
            // Show just the context window even when no usage data yet.
            let text = format!("—/{}", format_tokens_compact(app.context_length));
            Line::from(Span::styled(text, Style::default().fg(Color::DarkGray)))
        }
    };
    f.render_widget(
        Paragraph::new(tokens_display).alignment(ratatui::layout::Alignment::Right),
        tokens_area,
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
        Style::default()
            .fg(accent)
            .add_modifier(Modifier::UNDERLINED),
    ));
    f.render_widget(
        Paragraph::new(credits_line).alignment(ratatui::layout::Alignment::Right),
        credits_area,
    );
    app.credits_header_rect = Some(credits_area);
}
