use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Collections ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    if app.collections.is_empty() {
        let text = Paragraph::new("No collections.\nPress Ctrl+N to create one.").block(block);
        frame.render_widget(text, area);
    } else {
        let items: Vec<String> = app
            .collections
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let marker = if Some(i) == app.selected_collection {
                    ">"
                } else {
                    " "
                };
                format!("{} {}", marker, c.name)
            })
            .collect();
        let text = Paragraph::new(items.join("\n")).block(block);
        frame.render_widget(text, area);
    }
}
