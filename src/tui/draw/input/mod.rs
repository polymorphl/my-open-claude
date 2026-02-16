//! Input section, welcome center, bottom bar, slash command autocomplete.

mod bar;
mod slash;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::super::constants::INPUT_LINES;

use crate::core::commands;

use super::super::app::App;
use super::super::constants::{ACCENT, SUGGESTIONS};
use super::welcome_mascot;

/// Fixed viewport height for the slash command autocomplete list (scrollable when more commands).
pub(crate) const AUTOCOMPLETE_VISIBLE_LINES: u16 = 6;

/// Width of the centered input when in welcome (no conversation) mode.
const WELCOME_INPUT_WIDTH: u16 = 64;

pub(crate) use bar::draw as draw_bottom_bar;

const ERROR_LINES: u16 = 2;

fn truncate_with_ellipsis(s: &str, max_width: usize) -> String {
    if s.chars().count() <= max_width {
        s.to_string()
    } else {
        format!(
            "{}…",
            s.chars()
                .take(max_width.saturating_sub(1))
                .collect::<String>()
        )
    }
}

pub(crate) fn draw_welcome_center(f: &mut Frame, app: &mut App, area: Rect) {
    let in_slash = app.input.starts_with('/');
    let filter = app.input.get(1..).unwrap_or("");
    let filtered = commands::filter_commands(filter);
    let ac_height = if in_slash && !filtered.is_empty() {
        AUTOCOMPLETE_VISIBLE_LINES
    } else {
        0
    };
    let has_error = app.credits_fetch_error.is_some();
    let base = 1 + INPUT_LINES + 1 + 1;
    let error_height = if has_error { ERROR_LINES } else { 0u16 };
    let total_height = area.height;
    let mascot_height = if ac_height > 0 {
        (total_height
            .saturating_sub(ac_height)
            .saturating_sub(base)
            .saturating_sub(error_height))
        .max(4)
    } else {
        (total_height
            .saturating_sub(base)
            .saturating_sub(error_height))
        .max(4)
    };

    let constraints: &[Constraint] = if ac_height > 0 {
        if has_error {
            &[
                Constraint::Length(mascot_height),
                Constraint::Length(error_height),
                Constraint::Length(1),
                Constraint::Length(ac_height),
                Constraint::Length(INPUT_LINES),
                Constraint::Length(1),
                Constraint::Length(1),
            ]
        } else {
            &[
                Constraint::Length(mascot_height),
                Constraint::Length(1),
                Constraint::Length(ac_height),
                Constraint::Length(INPUT_LINES),
                Constraint::Length(1),
                Constraint::Length(1),
            ]
        }
    } else if has_error {
        &[
            Constraint::Length(mascot_height),
            Constraint::Length(error_height),
            Constraint::Length(1),
            Constraint::Length(INPUT_LINES),
            Constraint::Length(1),
            Constraint::Length(1),
        ]
    } else {
        &[
            Constraint::Length(mascot_height),
            Constraint::Length(1),
            Constraint::Length(INPUT_LINES),
            Constraint::Length(1),
            Constraint::Length(1),
        ]
    };

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let (input_area_outer, suggestions_area, error_area) = if ac_height > 0 {
        if has_error {
            (inner_chunks[4], inner_chunks[5], Some(inner_chunks[1]))
        } else {
            (inner_chunks[3], inner_chunks[4], None)
        }
    } else if has_error {
        (inner_chunks[3], inner_chunks[4], Some(inner_chunks[1]))
    } else {
        (inner_chunks[2], inner_chunks[3], None)
    };

    welcome_mascot::draw_mascot(f, inner_chunks[0]);

    if let (Some(area), Some(err)) = (error_area, app.credits_fetch_error.as_ref()) {
        let err_line = Line::from(Span::styled(
            truncate_with_ellipsis(err, area.width as usize),
            Style::default().fg(Color::Red),
        ));
        f.render_widget(
            Paragraph::new(err_line).alignment(ratatui::layout::HorizontalAlignment::Center),
            area,
        );
    }

    let input_width = WELCOME_INPUT_WIDTH.min(area.width);
    let input_area = Rect {
        x: area.x + area.width.saturating_sub(input_width) / 2,
        y: input_area_outer.y,
        width: input_width,
        height: input_area_outer.height,
    };

    if ac_height > 0 {
        let ac_area = if has_error {
            inner_chunks[3]
        } else {
            inner_chunks[2]
        };
        // Welcome mode: keep centered narrow layout to avoid misalignment with mascot/input.
        let ac_rect = Rect {
            x: area.x + area.width.saturating_sub(input_width) / 2,
            y: ac_area.y,
            width: input_width,
            height: ac_area.height,
        };
        slash::draw(f, app, ac_rect);
    }

    draw_input_block(f, app, input_area);

    let suggestion_spans = build_suggestion_spans(app);
    f.render_widget(
        Paragraph::new(Line::from(suggestion_spans))
            .alignment(ratatui::layout::HorizontalAlignment::Center),
        suggestions_area,
    );
}

fn build_suggestion_spans(app: &App) -> Vec<Span<'_>> {
    let mut spans: Vec<Span> = Vec::new();
    let sep = Span::styled(" · ", Style::default().fg(Color::DarkGray));
    for (i, s) in SUGGESTIONS.iter().enumerate() {
        if i > 0 {
            spans.push(sep.clone());
        }
        let selected = i == app.selected_suggestion;
        spans.push(Span::styled(
            format!(" {} ", s),
            if selected {
                Style::default().fg(Color::Black).bg(ACCENT)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ));
    }
    spans
}

fn wrapped_lines(text: &str, width: u16) -> Vec<String> {
    if width == 0 {
        return vec![];
    }
    textwrap::wrap(text, width as usize)
        .into_iter()
        .map(|s| s.into_owned())
        .collect()
}

fn input_has_focus(app: &App) -> bool {
    app.confirm_popup.is_none() && app.model_selector.is_none() && app.history_selector.is_none()
}

fn draw_input_block(f: &mut Frame, app: &mut App, input_area: Rect) {
    let border_style = if input_has_focus(app) {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = input_block.inner(input_area);
    let inner_height = inner.height as usize;

    let input_content = if app.input.is_empty() {
        Span::styled("Ask anything... ", Style::default().fg(Color::DarkGray))
    } else {
        Span::raw(app.input.as_str())
    };

    let para = Paragraph::new(Line::from(input_content))
        .block(input_block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });

    let lines = wrapped_lines(app.input.as_str(), inner.width);
    let total_lines = lines.len().max(1);

    // Must be at char boundary or str[..n] panics (UTF-8 multi-byte chars: é, 你, emoji).
    let cursor_byte = app
        .input
        .floor_char_boundary(app.input_cursor.min(app.input.len()));
    let cursor_char_offset = app.input[..cursor_byte].chars().count();
    let (cursor_line, cursor_col) = {
        let mut idx = 0;
        let mut found = (0, 0);
        for (i, line) in lines.iter().enumerate() {
            let len = line.chars().count();
            if cursor_char_offset <= idx + len {
                found = (i, (cursor_char_offset - idx).min(line.chars().count()));
                break;
            }
            idx += len;
        }
        if cursor_char_offset >= idx {
            let last = lines.last().map(|s| s.chars().count()).unwrap_or(0);
            found = (total_lines.saturating_sub(1), last);
        }
        found
    };
    let scroll_y = cursor_line
        .saturating_sub(inner_height.saturating_sub(1))
        .min(total_lines.saturating_sub(inner_height));
    let para = para.scroll((scroll_y as u16, 0));

    f.render_widget(para, input_area);

    let cursor_row_in_view = cursor_line.saturating_sub(scroll_y);
    let cx = inner.x + cursor_col.min(inner.width as usize) as u16;
    let cy = inner.y + cursor_row_in_view as u16;
    f.set_cursor_position(Position::new(cx, cy));
}

fn draw_suggestions(f: &mut Frame, app: &mut App, area: Rect) {
    let suggestion_spans = build_suggestion_spans(app);
    f.render_widget(
        Paragraph::new(Line::from(suggestion_spans))
            .alignment(ratatui::layout::HorizontalAlignment::Center),
        area,
    );
}

pub(crate) fn draw_input_section(f: &mut Frame, app: &mut App, input_section: Rect) {
    let in_slash = app.input.starts_with('/');
    let filter = app.input.get(1..).unwrap_or("");
    let filtered = commands::filter_commands(filter);
    let ac_height = if in_slash && !filtered.is_empty() {
        AUTOCOMPLETE_VISIBLE_LINES
    } else {
        0
    };

    let constraints: &[Constraint] = if ac_height > 0 {
        &[
            Constraint::Length(ac_height),
            Constraint::Length(INPUT_LINES),
            Constraint::Length(1),
            Constraint::Length(2),
        ]
    } else {
        &[
            Constraint::Length(INPUT_LINES),
            Constraint::Length(1),
            Constraint::Length(2),
        ]
    };

    let input_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(input_section);

    let input_area = if ac_height > 0 {
        input_chunks[1]
    } else {
        input_chunks[0]
    };
    let suggestions_area = if ac_height > 0 {
        input_chunks[2]
    } else {
        input_chunks[1]
    };
    let shortcuts_area = if ac_height > 0 {
        input_chunks[3]
    } else {
        input_chunks[2]
    };

    if ac_height > 0 {
        slash::draw(f, app, input_chunks[0]);
    }
    draw_input_block(f, app, input_area);
    draw_suggestions(f, app, suggestions_area);
    bar::draw(f, app, shortcuts_area);
}

#[cfg(test)]
mod tests {
    use super::truncate_with_ellipsis;

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 10), "hello");
    }

    #[test]
    fn truncate_exact_width_unchanged() {
        assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    }

    #[test]
    fn truncate_long_string_adds_ellipsis() {
        let result = truncate_with_ellipsis("hello world", 8);
        assert_eq!(result.chars().count(), 8); // 7 chars + ellipsis
        assert!(result.ends_with('…'));
    }

    #[test]
    fn truncate_utf8_chars() {
        let result = truncate_with_ellipsis("café", 3);
        assert!(result.ends_with('…'));
        assert!(result.len() <= 5); // é is 2 bytes
    }

    #[test]
    fn truncate_max_width_one() {
        let result = truncate_with_ellipsis("ab", 1);
        assert_eq!(result, "…");
    }
}
