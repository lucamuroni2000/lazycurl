use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, InputMode};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.input_mode {
        InputMode::Normal => Span::styled(
            " NORMAL ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::Editing => Span::styled(
            " EDITING ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    };

    let status_msg = if let Some(msg) = &app.status_message {
        Span::styled(format!(" {} ", msg), Style::default().fg(Color::White))
    } else {
        Span::raw("")
    };

    let hints = match app.input_mode {
        InputMode::Normal => Span::styled(
            " Tab:pane  Enter:edit  ?:help  Ctrl+Q:quit ",
            Style::default().fg(Color::DarkGray),
        ),
        InputMode::Editing => Span::styled(
            " Esc:done  Ctrl+Enter:send  Tab:next pane ",
            Style::default().fg(Color::DarkGray),
        ),
    };

    let line = Line::from(vec![mode_indicator, Span::raw(" "), status_msg, hints]);

    let status = Paragraph::new(line).style(Style::default().bg(Color::Black));
    frame.render_widget(status, area);
}
