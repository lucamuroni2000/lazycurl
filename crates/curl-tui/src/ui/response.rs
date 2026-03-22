use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Wrap};
use ratatui::Frame;

use crate::app::{App, Pane, ResponseTab};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.active_pane == Pane::Response;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(" Response ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 {
        return;
    }

    // Split: status line | tabs | content
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);

    // Status line
    if let Some(resp) = &app.last_response {
        let status_color = match resp.status_code {
            200..=299 => Color::Green,
            300..=399 => Color::Yellow,
            400..=499 => Color::Red,
            500..=599 => Color::Magenta,
            _ => Color::White,
        };
        let status_line = Line::from(vec![
            Span::styled(
                format!(" {} ", resp.status_code),
                Style::default()
                    .fg(Color::Black)
                    .bg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                status_text(resp.status_code),
                Style::default().fg(status_color),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{:.0}ms", resp.timing.total_ms),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        frame.render_widget(Paragraph::new(status_line), chunks[0]);
    } else {
        frame.render_widget(
            Paragraph::new(" No response yet").style(Style::default().fg(Color::DarkGray)),
            chunks[0],
        );
    }

    // Tabs
    let tab_titles = vec!["Body", "Headers", "Timing"];
    let selected_tab = match app.response_tab {
        ResponseTab::Body => 0,
        ResponseTab::Headers => 1,
        ResponseTab::Timing => 2,
    };
    let tabs = Tabs::new(tab_titles)
        .select(selected_tab)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .divider("|");
    frame.render_widget(tabs, chunks[1]);

    // Tab content
    if let Some(resp) = &app.last_response {
        match app.response_tab {
            ResponseTab::Body => {
                let body_text = &resp.body;
                let paragraph = Paragraph::new(body_text.as_str())
                    .style(Style::default().fg(Color::White))
                    .wrap(Wrap { trim: false })
                    .scroll((app.response_scroll as u16, 0));
                frame.render_widget(paragraph, chunks[2]);
            }
            ResponseTab::Headers => {
                let mut lines = Vec::new();
                for (key, value) in &resp.headers {
                    lines.push(Line::from(vec![
                        Span::styled(key, Style::default().fg(Color::Yellow)),
                        Span::styled(": ", Style::default().fg(Color::DarkGray)),
                        Span::styled(value, Style::default().fg(Color::White)),
                    ]));
                }
                frame.render_widget(Paragraph::new(lines), chunks[2]);
            }
            ResponseTab::Timing => {
                let timing = &resp.timing;
                let lines = vec![
                    Line::from(format!(" DNS Lookup:   {:.1}ms", timing.dns_lookup_ms)),
                    Line::from(format!(" TCP Connect:  {:.1}ms", timing.tcp_connect_ms)),
                    Line::from(format!(" TLS Handshake:{:.1}ms", timing.tls_handshake_ms)),
                    Line::from(format!(" First Byte:   {:.1}ms", timing.transfer_start_ms)),
                    Line::from(format!(" Total:        {:.1}ms", timing.total_ms)),
                ];
                frame.render_widget(
                    Paragraph::new(lines).style(Style::default().fg(Color::White)),
                    chunks[2],
                );
            }
        }
    } else {
        frame.render_widget(
            Paragraph::new(" Send a request to see the response.")
                .style(Style::default().fg(Color::DarkGray)),
            chunks[2],
        );
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
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "",
    }
}
