//! Renders the file picker popup overlay.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use super::super::app::FilePickerState;
use super::super::constants::ACCENT;

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let vertical_areas = vertical.split(area);
    let horizontal_areas = horizontal.split(vertical_areas[0]);
    horizontal_areas[0]
}

/// Draw the file picker popup over the given area.
pub(crate) fn draw_file_picker_popup(f: &mut Frame, area: Rect, picker: &mut FilePickerState) {
    let popup_rect = popup_area(area, 60, 60);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(" Insert file reference (@) ");

    let inner = block.inner(popup_rect);
    f.render_widget(Clear, popup_rect);
    f.render_widget(block, popup_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // breadcrumb
            Constraint::Length(3), // filter input
            Constraint::Min(3),    // scrollable list
            Constraint::Length(1), // hint
        ])
        .split(inner);

    let breadcrumb_area = chunks[0];
    let filter_area = chunks[1];
    let list_area = chunks[2];
    let hint_area = chunks[3];

    // Breadcrumb: show path relative to workspace_root, or just "." if equal
    let rel = picker
        .current_dir
        .strip_prefix(&picker.workspace_root)
        .unwrap_or(std::path::Path::new("."));
    let crumb = rel.display().to_string();
    let crumb_display = if crumb.is_empty() {
        ".".to_string()
    } else {
        crumb
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            crumb_display,
            Style::default().fg(Color::DarkGray),
        ))),
        breadcrumb_area,
    );

    // Filter input box
    let filter_content = if picker.filter.is_empty() {
        Span::styled("Filter... ", Style::default().fg(Color::DarkGray))
    } else {
        Span::raw(picker.filter.as_str())
    };
    let filter_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let filter_inner = filter_block.inner(filter_area);
    let filter_para = Paragraph::new(Line::from(filter_content))
        .block(filter_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(filter_para, filter_area);

    // Place cursor inside the filter box
    let cx = filter_inner.x
        + picker
            .filter
            .chars()
            .count()
            .min(filter_inner.width as usize) as u16;
    let cy = filter_area.y + 1;
    f.set_cursor_position(ratatui::layout::Position::new(cx, cy));

    // File / directory list — clamp selected_index before borrowing entries for rendering.
    {
        let filtered_len = picker.filtered_entries().len();
        if filtered_len > 0 {
            picker.selected_index = picker.selected_index.min(filtered_len.saturating_sub(1));
        }
    }
    let filtered = picker.filtered_entries();

    if filtered.is_empty() {
        let para = Paragraph::new(Line::from(Span::styled(
            "No matches",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        )));
        f.render_widget(para, list_area);
    } else {
        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let label = if entry.is_dir {
                    format!(" {}/", entry.name)
                } else {
                    format!(" {}", entry.name)
                };
                let style = if i == picker.selected_index {
                    Style::default().fg(Color::Black).bg(ACCENT)
                } else if entry.is_dir {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                };
                ListItem::new(label).style(style)
            })
            .collect();

        picker.list_state.select(Some(picker.selected_index));

        let list = List::new(items).highlight_style(Style::default().fg(Color::Black).bg(ACCENT));
        f.render_stateful_widget(list, list_area, &mut picker.list_state);
    }

    // Hint bar
    let hint = Paragraph::new(Line::from(vec![
        Span::styled("↑↓ ", Style::default().fg(Color::DarkGray)),
        Span::raw("navigate  "),
        Span::styled("Enter ", Style::default().fg(Color::DarkGray)),
        Span::raw("select  "),
        Span::styled("Bksp ", Style::default().fg(Color::DarkGray)),
        Span::raw("up  "),
        Span::styled("Esc ", Style::default().fg(Color::DarkGray)),
        Span::raw("cancel  "),
        Span::styled("type ", Style::default().fg(Color::DarkGray)),
        Span::raw("filter"),
    ]));
    f.render_widget(hint, hint_area);
}
