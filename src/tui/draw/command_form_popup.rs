//! Draw create/update command form popup.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

use crate::core::templates::CustomTemplate;

use super::super::app::{CommandFormField, CommandFormPhase, CommandFormState};
use super::super::constants::ACCENT;

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let vertical_areas = vertical.split(area);
    let horizontal_areas = horizontal.split(vertical_areas[0]);
    horizontal_areas[0]
}

fn field_label(f: CommandFormField, value: &str, focused: bool) -> (String, bool) {
    let label = match f {
        CommandFormField::Name => "Name",
        CommandFormField::Description => "Description",
        CommandFormField::Prompt => "Prompt",
        CommandFormField::Mode => "Mode",
    };
    let display = if value.is_empty() && f != CommandFormField::Mode {
        format!("{}...", label)
    } else {
        value.to_string()
    };
    let text = if focused {
        format!("▸ {}: {}", label, display)
    } else {
        format!("  {}: {}", label, display)
    };
    (text, focused)
}

pub(crate) fn draw_command_form_popup(
    f: &mut Frame,
    area: Rect,
    state: &mut CommandFormState,
    custom_templates: &[CustomTemplate],
) {
    let (title, popup_rect) = match state.phase {
        CommandFormPhase::SelectCommand => {
            let rect = popup_area(area, 50, 40);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .title(" Update command - select one ");
            let inner = block.inner(rect);
            f.render_widget(Clear, rect);
            f.render_widget(block, rect);

            let items: Vec<ListItem> = custom_templates
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let style = if i == state.selected_index {
                        Style::default().fg(Color::Black).bg(ACCENT)
                    } else {
                        Style::default()
                    };
                    ListItem::new(format!(" /{} - {}", t.name, t.description)).style(style)
                })
                .collect();
            let list = List::new(items);
            f.render_widget(list, inner);

            let hint = Paragraph::new(Line::from(vec![
                Span::styled("↑↓ ", Style::default().fg(Color::DarkGray)),
                Span::raw("select  "),
                Span::styled("Enter ", Style::default().fg(Color::DarkGray)),
                Span::raw("edit  "),
                Span::styled("Esc ", Style::default().fg(Color::DarkGray)),
                Span::raw("cancel"),
            ]));
            let hint_rect = Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(1),
                width: inner.width,
                height: 1,
            };
            f.render_widget(hint, hint_rect);
            return;
        }
        CommandFormPhase::EditForm => {
            let title = match state.form_mode {
                crate::tui::app::CommandFormMode::Create => " Create command ",
                crate::tui::app::CommandFormMode::Update { .. } => " Update command ",
            };
            let rect = popup_area(area, 70, 55);
            (title, rect)
        }
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .title(title);
    let inner = block.inner(popup_rect);
    f.render_widget(Clear, popup_rect);
    f.render_widget(block, popup_rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(inner);

    let prompt_focused = state.focused_field == CommandFormField::Prompt;
    let prompt_display = field_label(
        CommandFormField::Prompt,
        &state.prompt_prefix,
        prompt_focused,
    )
    .0;

    let (name_str, name_focused) = field_label(
        CommandFormField::Name,
        &state.name,
        state.focused_field == CommandFormField::Name,
    );
    let (desc_str, desc_focused) = field_label(
        CommandFormField::Description,
        &state.description,
        state.focused_field == CommandFormField::Description,
    );
    let (mode_str, mode_focused) = field_label(
        CommandFormField::Mode,
        &state.llm_mode,
        state.focused_field == CommandFormField::Mode,
    );

    let focus_style = Style::default().fg(ACCENT).add_modifier(Modifier::BOLD);
    let normal_style = Style::default();

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            name_str,
            if name_focused {
                focus_style
            } else {
                normal_style
            },
        ))),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            desc_str,
            if desc_focused {
                focus_style
            } else {
                normal_style
            },
        ))),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            mode_str,
            if mode_focused {
                focus_style
            } else {
                normal_style
            },
        ))),
        chunks[2],
    );

    let prompt_para = Paragraph::new(Line::from(Span::styled(
        prompt_display,
        if prompt_focused {
            focus_style
        } else {
            normal_style
        },
    )))
    .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(prompt_para, chunks[3]);

    if let Some(ref err) = state.error {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                err.as_str(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ))),
            chunks[4],
        );
    }

    let hint = Paragraph::new(Line::from(vec![
        Span::styled("Tab ", Style::default().fg(Color::DarkGray)),
        Span::raw("next  "),
        Span::styled("Shift+Tab ", Style::default().fg(Color::DarkGray)),
        Span::raw("prev  "),
        Span::styled("Enter ", Style::default().fg(Color::DarkGray)),
        Span::raw("save  "),
        Span::styled("Esc ", Style::default().fg(Color::DarkGray)),
        Span::raw("cancel"),
    ]));
    f.render_widget(hint, chunks[5]);
}
