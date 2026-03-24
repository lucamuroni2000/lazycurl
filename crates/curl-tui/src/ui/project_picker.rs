use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup_width = (area.width * 50 / 100).max(30).min(area.width);
    let popup_height = (area.height * 60 / 100).max(10).min(area.height);
    let x = (area.width - popup_width) / 2;
    let y = (area.height - popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Projects — Enter: open  n: new  d: close  Esc: cancel ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let open_slugs: Vec<&str> = app
        .open_projects
        .iter()
        .map(|ws| ws.data.slug.as_str())
        .collect();

    let items: Vec<ListItem> = app
        .all_projects
        .iter()
        .enumerate()
        .map(|(i, (project, slug))| {
            let is_open = open_slugs.contains(&slug.as_str());
            let marker = if is_open { "* " } else { "  " };
            let style = if i == app.project_picker_cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if is_open {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(vec![
                Span::raw(marker),
                Span::styled(&project.name, style),
            ]))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
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
        .title(" Welcome to curl-tui! ")
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
