use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, RequestTab};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let tabs = format!(
        " {} | {} | {} | {} ",
        tab_label("Headers", app.request_tab == RequestTab::Headers),
        tab_label("Body", app.request_tab == RequestTab::Body),
        tab_label("Auth", app.request_tab == RequestTab::Auth),
        tab_label("Params", app.request_tab == RequestTab::Params),
    );

    let block = Block::default()
        .title(" Request ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let content = if let Some(req) = &app.current_request {
        format!(
            "[{}] {}\n{}\n\nEdit request details here...",
            req.method, req.url, tabs
        )
    } else {
        format!("{}\n\nNo request selected.", tabs)
    };

    let text = Paragraph::new(content).block(block);
    frame.render_widget(text, area);
}

fn tab_label(name: &str, active: bool) -> String {
    if active {
        format!("[{}]", name)
    } else {
        name.to_string()
    }
}
