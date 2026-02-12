//! Input section, welcome center, bottom bar, slash command autocomplete.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use std::env;

use super::super::constants::INPUT_LINES;

use crate::core::commands;

use super::super::app::App;
use super::super::constants::{ACCENT, ACCENT_SECONDARY, SUGGESTIONS};
use super::welcome_mascot;

/// Fixed viewport height for the slash command autocomplete list (scrollable when more commands).
pub(crate) const AUTOCOMPLETE_VISIBLE_LINES: u16 = 6;

/// Width of the centered input when in welcome (no conversation) mode.
const WELCOME_INPUT_WIDTH: u16 = 64;

/// Draw the slash command autocomplete list above the given area.
/// List is scrollable when there are more commands than the visible viewport.
fn draw_slash_autocomplete(f: &mut Frame, app: &App, area: Rect) {
    if !app.input.starts_with('/') {
        return;
    }
    let filter = app.input.get(1..).unwrap_or("");
    let filtered = commands::filter_commands(filter);
    if filtered.is_empty() {
        return;
    }
    let total = filtered.len();
    let visible = AUTOCOMPLETE_VISIBLE_LINES as usize;
    // Keep selected item in view when scrolling
    let scroll_start = app
        .selected_command_index
        .saturating_sub(visible.saturating_sub(1))
        .min(total.saturating_sub(visible).max(0));
    let scroll_end = (scroll_start + visible).min(total);

    let lines: Vec<Line> = filtered[scroll_start..scroll_end]
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let idx = scroll_start + i;
            let selected = idx == app.selected_command_index;
            let name = cmd.full_name();
            let desc = format!("  {}", cmd.description);
            if selected {
                Line::from(vec![
                    Span::styled(name, Style::default().fg(Color::Black).bg(ACCENT)),
                    Span::styled(desc, Style::default().fg(Color::Black).bg(ACCENT)),
                ])
            } else {
                Line::from(vec![
                    Span::styled(name, Style::default().fg(ACCENT_SECONDARY)),
                    Span::styled(desc, Style::default().fg(Color::DarkGray)),
                ])
            }
        })
        .collect();

    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines), inner);
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
    // Fit within the 35-line welcome area (spacer + ac? + input + suggestions + spacer).
    let base = 1 + INPUT_LINES + 1 + 1; // spacer + input + suggestions + spacer
    let mascot_height = if ac_height > 0 {
        (35u16.saturating_sub(ac_height).saturating_sub(base)).max(10)
    } else {
        35u16.saturating_sub(base)
    };

    let constraints: &[Constraint] = if ac_height > 0 {
        &[
            Constraint::Length(mascot_height),
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
            Constraint::Length(INPUT_LINES),
            Constraint::Length(1),
            Constraint::Length(1),
        ]
    };

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mascot_area = inner_chunks[0];
    let input_area_outer = if ac_height > 0 {
        inner_chunks[3]
    } else {
        inner_chunks[2]
    };
    let suggestions_area = if ac_height > 0 {
        inner_chunks[4]
    } else {
        inner_chunks[3]
    };

    welcome_mascot::draw_mascot(f, mascot_area);

    let input_width = WELCOME_INPUT_WIDTH.min(area.width);
    let input_area = Rect {
        x: area.x + area.width.saturating_sub(input_width) / 2,
        y: input_area_outer.y,
        width: input_width,
        height: input_area_outer.height,
    };

    if ac_height > 0 {
        let ac_area = inner_chunks[2];
        let ac_rect = Rect {
            x: area.x + area.width.saturating_sub(input_width) / 2,
            y: ac_area.y,
            width: input_width,
            height: ac_area.height,
        };
        draw_slash_autocomplete(f, app, ac_rect);
    }

    draw_input_block(f, app, input_area);

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
    f.render_widget(
        Paragraph::new(Line::from(suggestion_spans)).alignment(ratatui::layout::Alignment::Center),
        suggestions_area,
    );
}

/// Wrapped lines for the given text and width.
fn wrapped_lines(text: &str, width: u16) -> Vec<String> {
    if width == 0 {
        return vec![];
    }
    textwrap::wrap(text, width as usize)
        .into_iter()
        .map(|s| s.into_owned())
        .collect()
}

/// Draw the input block (multi-line textarea with wrap) and set cursor position.
fn draw_input_block(f: &mut Frame, app: &mut App, input_area: Rect) {
    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
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
    let scroll_y = total_lines.saturating_sub(inner_height);
    let para = para.scroll((scroll_y as u16, 0));

    f.render_widget(para, input_area);

    let cursor_line = total_lines - 1;
    let cursor_row_in_view = cursor_line.saturating_sub(scroll_y);
    let cursor_col = lines
        .last()
        .map(|s| s.chars().count())
        .unwrap_or(0)
        .min(inner.width as usize);
    let cy = inner.y + cursor_row_in_view as u16;
    let cx = inner.x + cursor_col as u16;
    f.set_cursor_position(Position::new(cx, cy));
}

/// Draw suggestions row.
fn draw_suggestions(f: &mut Frame, app: &mut App, area: Rect) {
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
    f.render_widget(
        Paragraph::new(Line::from(suggestion_spans)).alignment(ratatui::layout::Alignment::Center),
        area,
    );
}

/// Bottom bar: path on left, shortcuts on right.
pub(crate) fn draw_bottom_bar(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(1), Constraint::Length(100)])
        .split(area);
    let path_area = chunks[0];
    let shortcuts_area = chunks[1];

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
    f.render_widget(
        Paragraph::new(path_line).alignment(ratatui::layout::Alignment::Left),
        path_area,
    );

    let shortcuts = super::super::shortcuts::labels::bottom_bar(app.is_streaming);
    f.render_widget(
        Paragraph::new(shortcuts).alignment(ratatui::layout::Alignment::Right),
        shortcuts_area,
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
            Constraint::Length(1),
        ]
    } else {
        &[
            Constraint::Length(INPUT_LINES),
            Constraint::Length(1),
            Constraint::Length(1),
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
        draw_slash_autocomplete(f, app, input_chunks[0]);
    }
    draw_input_block(f, app, input_area);
    draw_suggestions(f, app, suggestions_area);
    draw_bottom_bar(f, app, shortcuts_area);
}
