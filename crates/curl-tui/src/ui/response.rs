use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
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
    if let Some(resp) = app.last_response() {
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
    let selected_tab = match app.response_tab() {
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
    if let Some(resp) = app.last_response() {
        match app.response_tab() {
            ResponseTab::Body => {
                let content_type = detect_content_type(resp);
                let lines = match content_type {
                    ContentType::Json => colorize_json(&resp.body),
                    ContentType::PlainText | ContentType::Other => resp
                        .body
                        .lines()
                        .map(|l| Line::from(l.to_string()))
                        .collect(),
                };

                // Show content type indicator + scroll info
                let type_label = match content_type {
                    ContentType::Json => "JSON",
                    ContentType::PlainText => "Text",
                    ContentType::Other => "Raw",
                };
                let total_lines = lines.len();
                let header_line = Line::from(vec![
                    Span::styled(
                        format!(" {} ", type_label),
                        Style::default().fg(Color::Black).bg(Color::DarkGray),
                    ),
                    Span::styled(
                        format!(
                            "  {} lines  (scroll: {}/{})",
                            total_lines,
                            app.response_scroll().min(total_lines),
                            total_lines
                        ),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]);

                // Split body area: type indicator (1 line) + body content (rest)
                let body_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Min(1)])
                    .split(chunks[2]);

                frame.render_widget(Paragraph::new(header_line), body_chunks[0]);
                frame.render_widget(
                    Paragraph::new(lines).scroll((app.response_scroll() as u16, 0)),
                    body_chunks[1],
                );
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

enum ContentType {
    Json,
    PlainText,
    Other,
}

fn detect_content_type(resp: &curl_tui_core::types::CurlResponse) -> ContentType {
    // Always check body content first — many servers return wrong Content-Type
    let trimmed = resp.body.trim();
    let body_looks_like_json = (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'));

    // If body looks like JSON, try to actually parse it to confirm
    if body_looks_like_json && serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return ContentType::Json;
    }

    // Fall back to Content-Type header
    for (key, value) in &resp.headers {
        if key.eq_ignore_ascii_case("content-type") {
            let v = value.to_lowercase();
            if v.contains("json") {
                return ContentType::Json;
            }
            if v.contains("text/html") || v.contains("text/xml") || v.contains("application/xml") {
                return ContentType::Other;
            }
        }
    }

    // If body looks like JSON but failed to parse (malformed), still try colorizing
    if body_looks_like_json {
        return ContentType::Json;
    }

    ContentType::PlainText
}

/// Pretty-print and syntax-highlight JSON.
/// Handles: single JSON value, JSON Lines (one object per line), and malformed fallback.
/// Colors: keys=yellow, strings=green, numbers=cyan, booleans=magenta, null=red, punctuation=darkgray
fn colorize_json(raw: &str) -> Vec<Line<'static>> {
    let trimmed = raw.trim();

    // 1. Try parsing as a single JSON value
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
        let pretty = serde_json::to_string_pretty(&val).unwrap_or_else(|_| trimmed.to_string());
        return pretty.lines().map(colorize_json_line).collect();
    }

    // 2. Try parsing as JSON Lines (one JSON object per line)
    let lines: Vec<&str> = trimmed.lines().collect();
    let mut all_json = !lines.is_empty();
    for line in &lines {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if serde_json::from_str::<serde_json::Value>(l).is_err() {
            all_json = false;
            break;
        }
    }

    if all_json {
        let mut result = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let l = line.trim();
            if l.is_empty() {
                result.push(Line::raw("".to_string()));
                continue;
            }
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(l) {
                let pretty = serde_json::to_string_pretty(&val).unwrap_or_else(|_| l.to_string());
                // Add a separator between JSONL entries
                if i > 0 {
                    result.push(Line::styled(
                        "---".to_string(),
                        Style::default().fg(Color::DarkGray),
                    ));
                }
                result.extend(pretty.lines().map(colorize_json_line));
            }
        }
        return result;
    }

    // 3. Fallback: show raw text, line by line
    trimmed.lines().map(|l| Line::from(l.to_string())).collect()
}

fn colorize_json_line(line: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();

    // Add indentation
    if indent > 0 {
        spans.push(Span::raw(" ".repeat(indent)));
    }

    let mut chars = trimmed.chars().peekable();
    let mut buf = String::new();

    while let Some(&c) = chars.peek() {
        match c {
            '"' => {
                // Collect the full quoted string
                let mut s = String::new();
                s.push(chars.next().unwrap()); // opening "
                let mut escaped = false;
                for ch in chars.by_ref() {
                    s.push(ch);
                    if escaped {
                        escaped = false;
                    } else if ch == '\\' {
                        escaped = true;
                    } else if ch == '"' {
                        break;
                    }
                }

                // Check if this is a key (followed by ':')
                let rest: String = chars.clone().collect();
                let rest_trimmed = rest.trim_start();
                if rest_trimmed.starts_with(':') {
                    // It's a key
                    spans.push(Span::styled(s, Style::default().fg(Color::Yellow)));
                } else {
                    // It's a string value
                    spans.push(Span::styled(s, Style::default().fg(Color::Green)));
                }
            }
            ':' => {
                chars.next();
                spans.push(Span::styled(": ", Style::default().fg(Color::DarkGray)));
                // Skip the space after colon if present
                if chars.peek() == Some(&' ') {
                    chars.next();
                }
            }
            '{' | '}' | '[' | ']' => {
                chars.next();
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
            ',' => {
                chars.next();
                spans.push(Span::styled(",", Style::default().fg(Color::DarkGray)));
            }
            't' | 'f' => {
                // true / false
                buf.clear();
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphabetic() {
                        buf.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if buf == "true" || buf == "false" {
                    spans.push(Span::styled(
                        buf.clone(),
                        Style::default().fg(Color::Magenta),
                    ));
                } else {
                    spans.push(Span::styled(buf.clone(), Style::default().fg(Color::White)));
                }
            }
            'n' => {
                // null
                buf.clear();
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphabetic() {
                        buf.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if buf == "null" {
                    spans.push(Span::styled(buf.clone(), Style::default().fg(Color::Red)));
                } else {
                    spans.push(Span::styled(buf.clone(), Style::default().fg(Color::White)));
                }
            }
            '0'..='9' | '-' => {
                // Number
                buf.clear();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_digit()
                        || ch == '.'
                        || ch == '-'
                        || ch == 'e'
                        || ch == 'E'
                        || ch == '+'
                    {
                        buf.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                spans.push(Span::styled(buf.clone(), Style::default().fg(Color::Cyan)));
            }
            ' ' => {
                chars.next();
                spans.push(Span::raw(" "));
            }
            _ => {
                chars.next();
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(Color::White),
                ));
            }
        }
    }

    Line::from(spans)
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
