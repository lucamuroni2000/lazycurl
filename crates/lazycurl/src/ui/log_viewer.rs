use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Reserve bottom row for the status bar (rendered by statusbar.rs)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);
    let content_area = outer[0];

    // Clear only the content area (status bar stays)
    frame.render_widget(Clear, content_area);

    if app.log_viewer_show_detail {
        // Split: top 40% list, bottom 60% detail
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(content_area);
        draw_list(frame, app, chunks[0]);
        draw_detail(frame, app, chunks[1]);
    } else {
        draw_list(frame, app, content_area);
    }
}

fn draw_list(frame: &mut Frame, app: &App, area: Rect) {
    let filtered = app.filtered_log_entries();

    let title = if app.log_viewer_editing_search {
        format!(
            " Request Log — Search: {}_ ",
            app.log_viewer_search_input.content()
        )
    } else if app.log_viewer_editing_filter {
        format!(
            " Request Log — Filter: {}_ ",
            app.log_viewer_filter_input.content()
        )
    } else if !app.log_viewer_search.is_empty() {
        format!(" Request Log — Search: {} ", app.log_viewer_search)
    } else if !app.log_viewer_filter.is_empty() {
        format!(" Request Log — Filter: {} ", app.log_viewer_filter)
    } else {
        " Request Log ".to_string()
    };

    let block = Block::default().title(title).borders(Borders::ALL);

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let time = entry.timestamp.format("%H:%M:%S").to_string();
            let method = format!("{:7}", entry.request.method);
            let (status, status_color) = match &entry.response {
                Some(resp) => {
                    let color = match resp.status_code / 100 {
                        2 => Color::Green,
                        3 => Color::Yellow,
                        4 | 5 => Color::Red,
                        _ => Color::White,
                    };
                    (format!("{:>3}", resp.status_code), color)
                }
                None => ("ERR".to_string(), Color::Red),
            };
            let duration = entry
                .response
                .as_ref()
                .map(|r| format!("{:>5}ms", r.time_ms))
                .unwrap_or_else(|| "   ---".to_string());

            let marker = if i == app.log_viewer_cursor { ">" } else { " " };

            let style = if i == app.log_viewer_cursor {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            // Highlight search matches in the URL
            let url_span = if !app.log_viewer_search.is_empty()
                && entry
                    .request
                    .url
                    .to_lowercase()
                    .contains(&app.log_viewer_search.to_lowercase())
            {
                Span::styled(
                    entry.request.url.clone(),
                    Style::default().fg(Color::Black).bg(Color::Yellow),
                )
            } else {
                Span::raw(entry.request.url.clone())
            };

            let line = Line::from(vec![
                Span::raw(format!("{} {}  ", marker, time)),
                Span::styled(method, Style::default()),
                Span::raw("  "),
                Span::styled(status, Style::default().fg(status_color)),
                Span::raw(format!("  {}  ", duration)),
                url_span,
            ]);

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn draw_detail(frame: &mut Frame, app: &App, area: Rect) {
    let filtered = app.filtered_log_entries();
    let entry = match filtered.get(app.log_viewer_cursor) {
        Some(e) => e,
        None => {
            let block = Block::default()
                .title(" Request Detail ")
                .borders(Borders::ALL);
            frame.render_widget(block, area);
            return;
        }
    };

    let mut lines: Vec<Line> = Vec::new();

    // Request line
    lines.push(Line::from(vec![
        Span::styled(
            entry.request.method.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" {}", entry.request.url)),
    ]));
    lines.push(Line::raw(""));

    // Request headers
    if !entry.request.headers.is_empty() {
        lines.push(Line::styled(
            "-- Request Headers --",
            Style::default().add_modifier(Modifier::DIM),
        ));
        for h in &entry.request.headers {
            lines.push(Line::raw(format!("{}: {}", h.name, h.value)));
        }
        lines.push(Line::raw(""));
    }

    // Request body
    if let Some(ref body) = entry.request.body {
        lines.push(Line::styled(
            "-- Request Body --",
            Style::default().add_modifier(Modifier::DIM),
        ));
        for body_line in body.lines() {
            lines.push(Line::raw(body_line.to_string()));
        }
        lines.push(Line::raw(""));
    }

    // Response
    if let Some(ref resp) = entry.response {
        if !resp.headers.is_empty() {
            lines.push(Line::styled(
                "-- Response Headers --",
                Style::default().add_modifier(Modifier::DIM),
            ));
            for h in &resp.headers {
                lines.push(Line::raw(format!("{}: {}", h.name, h.value)));
            }
            lines.push(Line::raw(""));
        }

        let body_label = if resp.body_truncated {
            format!(
                "-- Response Body ({} bytes, truncated) --",
                resp.body_size_bytes
            )
        } else {
            format!("-- Response Body ({} bytes) --", resp.body_size_bytes)
        };
        lines.push(Line::styled(
            body_label,
            Style::default().add_modifier(Modifier::DIM),
        ));
        if let Some(ref body) = resp.body {
            for body_line in body.lines() {
                lines.push(Line::raw(body_line.to_string()));
            }
        } else if resp.body_type == "binary" {
            lines.push(Line::raw("[binary content not stored]"));
        }
    }

    if let Some(ref err) = entry.error {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red),
        ));
    }

    let block = Block::default()
        .title(" Request Detail ")
        .borders(Borders::ALL);
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}
