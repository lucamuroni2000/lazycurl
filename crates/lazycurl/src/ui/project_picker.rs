use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_width = (area.width * 50 / 100).max(30).min(area.width);
    let confirm_line = if app.project_picker_confirm_delete.is_some() {
        1
    } else {
        0
    };
    let content_lines = app.all_projects.len().max(1) + confirm_line;
    let popup_height = ((content_lines + 2) as u16)
        .max(5)
        .min(area.height * 60 / 100)
        .min(area.height);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Projects — Enter:open  n:new  r:rename  c:close  d:delete  Esc:cancel ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let open_slugs: Vec<&str> = app
        .open_projects
        .iter()
        .map(|ws| ws.data.slug.as_str())
        .collect();

    // If there's a confirm delete, reserve the last line
    let list_area = if app.project_picker_confirm_delete.is_some() && inner.height > 1 {
        Rect::new(inner.x, inner.y, inner.width, inner.height - 1)
    } else {
        inner
    };

    let items: Vec<ListItem> = app
        .all_projects
        .iter()
        .enumerate()
        .map(|(i, (project, slug))| {
            let is_open = open_slugs.contains(&slug.as_str());
            let is_cursor = i == app.project_picker_cursor;
            let is_renaming = app.project_picker_renaming && is_cursor;
            let marker = if is_open { "* " } else { "  " };

            let name_span = if is_renaming {
                Span::styled(
                    app.project_picker_name_input.content(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::UNDERLINED),
                )
            } else {
                let style = if is_cursor {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if is_open {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::White)
                };
                Span::styled(&project.name, style)
            };

            ListItem::new(Line::from(vec![Span::raw(marker), name_span]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, list_area);

    // Show cursor position when renaming
    if app.project_picker_renaming {
        // marker (2) + input cursor
        let cursor_x = inner.x + 2 + app.project_picker_name_input.cursor() as u16;
        let cursor_y = inner.y + app.project_picker_cursor as u16;
        if cursor_x < inner.x + inner.width && cursor_y < inner.y + inner.height {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    // Delete confirmation line
    if let Some(delete_idx) = app.project_picker_confirm_delete {
        if let Some((project, _)) = app.all_projects.get(delete_idx) {
            let confirm_area = Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
            let msg = Paragraph::new(Line::from(vec![
                Span::styled(
                    format!(" Delete '{}'? ", project.name),
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

pub fn draw_first_launch(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let w = 50u16.min(area.width);
    let h = 5u16.min(area.height);
    let x = (area.width - w) / 2;
    let y = (area.height - h) / 2;
    let popup = Rect::new(x, y, w, h);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .title(" Welcome to lazycurl! ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let label = Span::styled("Project name: ", Style::default().fg(Color::Yellow));
    let content = Span::raw(app.name_input.content());
    frame.render_widget(Paragraph::new(Line::from(vec![label, content])), inner);

    // Show cursor
    let cursor_x = inner.x + 14 + app.name_input.cursor() as u16;
    if cursor_x < inner.x + inner.width {
        frame.set_cursor_position((cursor_x, inner.y));
    }
}
