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

    let title = {
        let mut parts = vec![" Request Log".to_string()];

        // Show active filter (or editing state)
        if app.log_viewer_editing_filter {
            parts.push(format!(
                "Filter: {}_",
                app.log_viewer_filter_input.content()
            ));
        } else if !app.log_viewer_filter.is_empty() {
            parts.push(format!("Filter: {}", app.log_viewer_filter));
        }

        // Show active search (or editing state)
        if app.log_viewer_editing_search {
            parts.push(format!(
                "Search: {}_",
                app.log_viewer_search_input.content()
            ));
        } else if !app.log_viewer_search.is_empty() {
            parts.push(format!("Search: {}", app.log_viewer_search));
        }

        if parts.len() == 1 {
            " Request Log ".to_string()
        } else {
            format!("{} ", parts.join(" — "))
        }
    };

    let list_focused = !app.log_viewer_detail_focused;
    let border_style = if list_focused && app.log_viewer_show_detail {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

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

    let section_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let header_key_style = Style::default().fg(Color::Yellow);
    let header_val_style = Style::default().fg(Color::White);
    let body_style = Style::default().fg(Color::Green);
    let redacted_style = Style::default().fg(Color::Red).add_modifier(Modifier::DIM);

    let mut lines: Vec<Line> = Vec::new();

    // Request line
    let method_color = match entry.request.method.to_string().as_str() {
        "GET" => Color::Green,
        "POST" => Color::Blue,
        "PUT" => Color::Yellow,
        "PATCH" => Color::Yellow,
        "DELETE" => Color::Red,
        _ => Color::Magenta,
    };
    lines.push(Line::from(vec![
        Span::styled(
            entry.request.method.to_string(),
            Style::default()
                .fg(method_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {}", entry.request.url),
            Style::default().fg(Color::White),
        ),
    ]));

    // Status + timing on same line
    if let Some(ref resp) = entry.response {
        let status_color = match resp.status_code / 100 {
            2 => Color::Green,
            3 => Color::Yellow,
            4 | 5 => Color::Red,
            _ => Color::White,
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} {}", resp.status_code, resp.status_text),
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}ms", resp.time_ms),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }
    lines.push(Line::raw(""));

    // Request headers
    if !entry.request.headers.is_empty() {
        lines.push(Line::styled("Request Headers", section_style));
        for h in &entry.request.headers {
            let val_style = if h.value.contains("[REDACTED]") {
                redacted_style
            } else {
                header_val_style
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {}: ", h.name), header_key_style),
                Span::styled(h.value.clone(), val_style),
            ]));
        }
        lines.push(Line::raw(""));
    }

    // Request body
    if let Some(ref body) = entry.request.body {
        lines.push(Line::styled("Request Body", section_style));
        append_formatted_body(&mut lines, body, body_style, redacted_style);
        lines.push(Line::raw(""));
    }

    // Response
    if let Some(ref resp) = entry.response {
        // Response headers
        if !resp.headers.is_empty() {
            lines.push(Line::styled("Response Headers", section_style));
            for h in &resp.headers {
                lines.push(Line::from(vec![
                    Span::styled(format!("  {}: ", h.name), header_key_style),
                    Span::styled(h.value.clone(), header_val_style),
                ]));
            }
            lines.push(Line::raw(""));
        }

        // Response body
        let body_label = if resp.body_truncated {
            format!("Response Body ({} bytes, truncated)", resp.body_size_bytes)
        } else {
            format!("Response Body ({} bytes)", resp.body_size_bytes)
        };
        lines.push(Line::styled(body_label, section_style));
        if let Some(ref body) = resp.body {
            append_formatted_body(&mut lines, body, body_style, redacted_style);
        } else if resp.body_type == "binary" {
            lines.push(Line::styled(
                "  [binary content not stored]",
                Style::default().fg(Color::DarkGray),
            ));
        }
    }

    // Curl command
    if !entry.curl_command.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled("Curl Command", section_style));
        lines.push(Line::styled(
            format!("  {}", entry.curl_command),
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Error
    if let Some(ref err) = entry.error {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            format!("Error: {}", err),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }

    let detail_border_style = if app.log_viewer_detail_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    let block = Block::default()
        .title(" Request Detail ")
        .borders(Borders::ALL)
        .border_style(detail_border_style);
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.log_viewer_detail_scroll as u16, 0));
    frame.render_widget(paragraph, area);
}

/// Append body lines to the output, pretty-printing JSON if possible.
fn append_formatted_body(
    lines: &mut Vec<Line<'static>>,
    body: &str,
    body_style: Style,
    redacted_style: Style,
) {
    // Try to parse and pretty-print as JSON
    let formatted = if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| body.to_string())
    } else {
        body.to_string()
    };

    for body_line in formatted.lines() {
        let style = if body_line.contains("[REDACTED]") {
            redacted_style
        } else {
            body_style
        };
        lines.push(Line::styled(format!("  {}", body_line), style));
    }
}
