pub mod collections;
pub mod help;
pub mod layout;
pub mod request;
pub mod response;
pub mod statusbar;

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let pane_layout = layout::compute_layout(frame.area(), app.pane_visible);

    // Title bar
    let env_name = app
        .active_environment
        .and_then(|i| app.environments.get(i))
        .map(|e| e.name.as_str())
        .unwrap_or("None");

    let title = Line::from(vec![
        Span::styled(" curl-tui", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled(
            format!("[env: {}]", env_name),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("  "),
        Span::styled("[v0.1.0]", Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(
        Paragraph::new(title).style(Style::default().bg(Color::Black)),
        pane_layout.title_bar,
    );

    // Panes
    if let Some(area) = pane_layout.collections {
        collections::draw(frame, app, area);
    }
    if let Some(area) = pane_layout.request {
        request::draw(frame, app, area);
    }
    if let Some(area) = pane_layout.response {
        response::draw(frame, app, area);
    }

    // Status bar
    statusbar::draw(frame, app, pane_layout.status_bar);

    // Help overlay (on top of everything)
    if app.show_help {
        help::draw(frame);
    }
}
