use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_width = (area.width * 50 / 100).max(40).min(area.width);
    // Height: 3 (border + padding) + env count + 1 (confirm line), capped at 60%
    let env_count = app
        .active_workspace()
        .map(|ws| ws.data.environments.len())
        .unwrap_or(0);
    let content_lines = if env_count == 0 { 1 } else { env_count };
    let confirm_line = if app.env_manager_confirm_delete.is_some() {
        1
    } else {
        0
    };
    let popup_height = ((content_lines + confirm_line + 2) as u16)
        .max(5)
        .min(area.height * 60 / 100)
        .min(area.height);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let project_name = app
        .active_workspace()
        .map(|ws| ws.data.project.name.as_str())
        .unwrap_or("No project");

    let title = format!(
        " Environments ({}) — n:new  r:rename  d:delete  Enter:activate  Esc:close ",
        project_name
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    if env_count == 0 {
        let msg = Paragraph::new(" No environments. Press n to create one.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(msg, inner);
        return;
    }

    let active_idx = app
        .active_workspace()
        .and_then(|ws| ws.data.active_environment);

    let environments: Vec<&curl_tui_core::types::Environment> = app
        .active_workspace()
        .map(|ws| ws.data.environments.iter().collect())
        .unwrap_or_default();

    // If there's a confirm delete, reserve the last line
    let list_area = if app.env_manager_confirm_delete.is_some() && inner.height > 1 {
        Rect::new(inner.x, inner.y, inner.width, inner.height - 1)
    } else {
        inner
    };

    let items: Vec<ListItem> = environments
        .iter()
        .enumerate()
        .map(|(i, env)| {
            let is_cursor = i == app.env_manager_cursor;
            let is_active = active_idx == Some(i);
            let is_renaming = app.env_manager_renaming == Some(i);

            let cursor_marker = if is_cursor { "> " } else { "  " };
            let active_marker = if is_active { "[*] " } else { "    " };

            let name_span = if is_renaming {
                // Show the text input content when renaming
                Span::styled(
                    app.env_manager_name_input.content(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::UNDERLINED),
                )
            } else {
                let style = if is_cursor {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                Span::styled(&env.name, style)
            };

            ListItem::new(Line::from(vec![
                Span::raw(cursor_marker),
                Span::styled(active_marker, Style::default().fg(Color::Green)),
                name_span,
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, list_area);

    // Show cursor position when renaming
    if let Some(rename_idx) = app.env_manager_renaming {
        if rename_idx < environments.len() {
            // cursor_marker (2) + active_marker (4) + input cursor
            let cursor_x = inner.x + 2 + 4 + app.env_manager_name_input.cursor() as u16;
            let cursor_y = inner.y + rename_idx as u16;
            if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }

    // Delete confirmation line
    if let Some(delete_idx) = app.env_manager_confirm_delete {
        if let Some(env) = environments.get(delete_idx) {
            let confirm_area = Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(
                    format!(" Delete '{}'? ", env.name),
                    Style::default().fg(Color::Red),
                ),
                Span::styled(
                    "y",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to confirm, ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    "Esc",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(" to cancel", Style::default().fg(Color::DarkGray)),
            ]));
            frame.render_widget(msg, confirm_area);
        }
    }
}
