use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let hints = vec![
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(":pane  "),
        Span::styled("Ctrl+Enter/F5", Style::default().fg(Color::Yellow)),
        Span::raw(":send  "),
        Span::styled("Ctrl+S", Style::default().fg(Color::Yellow)),
        Span::raw(":save  "),
        Span::styled("Ctrl+E", Style::default().fg(Color::Yellow)),
        Span::raw(":env  "),
        Span::styled("?", Style::default().fg(Color::Yellow)),
        Span::raw(":help  "),
        Span::styled("Ctrl+Q", Style::default().fg(Color::Yellow)),
        Span::raw(":quit"),
    ];

    let _ = app; // suppress unused warning until more fields are used
    let status = Paragraph::new(Line::from(hints))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status, area);
}
