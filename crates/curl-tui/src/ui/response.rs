use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, ResponseTab};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let tabs = format!(
        " {} | {} | {} ",
        tab_label("Body", app.response_tab == ResponseTab::Body),
        tab_label("Headers", app.response_tab == ResponseTab::Headers),
        tab_label("Timing", app.response_tab == ResponseTab::Timing),
    );

    let block = Block::default()
        .title(" Response ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let content = if let Some(resp) = &app.last_response {
        format!(
            "[{} {}] {:.0}ms\n{}\n\n{}",
            resp.status_code,
            status_text(resp.status_code),
            resp.timing.total_ms,
            tabs,
            &resp.body[..resp.body.len().min(500)]
        )
    } else {
        format!("{}\n\nSend a request to see the response.", tabs)
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

fn status_text(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "",
    }
}
