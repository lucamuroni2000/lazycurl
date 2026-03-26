use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Frame;

use crate::app::{App, EditField, InputMode, Pane, RequestTab};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.active_pane == Pane::Request;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(" Request ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 3 {
        return;
    }

    // Split inner area: name (1 line) | method+url (1 line) | tabs (1 line) | content (rest)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Request name
            Constraint::Length(1), // Method + URL
            Constraint::Length(1), // Tabs
            Constraint::Min(1),    // Tab content
        ])
        .split(inner);

    // Request name
    if let Some(req) = app.current_request() {
        let name_editing =
            app.input_mode == InputMode::Editing && app.edit_field == Some(EditField::RequestName);

        let name_text = if name_editing {
            app.name_input.content().to_string()
        } else {
            req.name.clone()
        };

        let name_style = if name_editing {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        };

        let name_line = Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(name_text.clone(), name_style),
            Span::styled("  (r: rename)", Style::default().fg(Color::DarkGray)),
        ]);
        frame.render_widget(Paragraph::new(name_line), chunks[0]);

        if name_editing {
            let cursor_x = chunks[0].x + 1 + app.name_input.cursor() as u16;
            let cursor_y = chunks[0].y;
            if cursor_x < chunks[0].x + chunks[0].width {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }

    // Method + URL bar
    if let Some(req) = app.current_request() {
        let url_editing =
            app.input_mode == InputMode::Editing && app.edit_field == Some(EditField::Url);

        let method_span = Span::styled(
            format!(" {} ", req.method),
            Style::default()
                .fg(method_color(req.method))
                .add_modifier(Modifier::BOLD),
        );

        let url_text = if url_editing {
            app.url_input.content()
        } else {
            &req.url
        };

        let url_style = if url_editing {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        } else if is_focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };

        let url_display = if url_text.is_empty() && !url_editing {
            "Enter URL...".to_string()
        } else {
            url_text.to_string()
        };

        let url_span = Span::styled(url_display, url_style);
        let line = Line::from(vec![method_span, Span::raw(" "), url_span]);
        frame.render_widget(Paragraph::new(line), chunks[1]);

        // Show cursor when editing URL
        if url_editing {
            let cursor_x = chunks[1].x
                + req.method.to_string().len() as u16
                + 3
                + app.url_input.cursor() as u16;
            let cursor_y = chunks[1].y;
            if cursor_x < chunks[1].x + chunks[1].width {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    } else {
        frame.render_widget(
            Paragraph::new("No request selected").style(Style::default().fg(Color::DarkGray)),
            chunks[1],
        );
    }

    // Tabs
    let tab_titles = vec!["Headers", "Body", "Auth", "Params"];
    let selected_tab = match app.request_tab() {
        RequestTab::Headers => 0,
        RequestTab::Body => 1,
        RequestTab::Auth => 2,
        RequestTab::Params => 3,
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
    frame.render_widget(tabs, chunks[2]);

    // Tab content
    match app.request_tab() {
        RequestTab::Headers => draw_headers(frame, app, chunks[3]),
        RequestTab::Body => draw_body(frame, app, chunks[3]),
        RequestTab::Auth => draw_auth(frame, app, chunks[3]),
        RequestTab::Params => draw_params(frame, app, chunks[3]),
    }
}

fn draw_headers(frame: &mut Frame, app: &App, area: Rect) {
    let Some(req) = app.current_request() else {
        return;
    };

    if req.headers.is_empty() {
        let text = Paragraph::new(" No headers. Press 'a' to add one.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(text, area);
        return;
    }

    let is_focused = app.active_pane == Pane::Request;
    let mut lines = Vec::new();
    let mut cursor_pos: Option<(u16, u16)> = None;

    for (i, header) in req.headers.iter().enumerate() {
        let is_selected = i == app.header_cursor && is_focused;
        let enabled = if header.enabled { " " } else { "x" };

        let editing_key = app.input_mode == InputMode::Editing
            && app.edit_field == Some(EditField::HeaderKey(i));
        let editing_value = app.input_mode == InputMode::Editing
            && app.edit_field == Some(EditField::HeaderValue(i));

        let key_display = if editing_key {
            app.header_key_inputs
                .get(i)
                .map(|inp| inp.content().to_string())
                .unwrap_or_default()
        } else {
            header.key.clone()
        };

        let value_display = if editing_value {
            app.header_value_inputs
                .get(i)
                .map(|inp| inp.content().to_string())
                .unwrap_or_default()
        } else {
            header.value.clone()
        };

        let base_style = if !header.enabled {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        let key_style = if editing_key {
            Style::default().fg(Color::Yellow).bg(Color::DarkGray)
        } else if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let value_style = if editing_value {
            base_style.bg(Color::DarkGray)
        } else if is_selected {
            base_style.add_modifier(Modifier::REVERSED)
        } else {
            base_style
        };

        let marker_style = if is_selected {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let sep_style = if is_selected {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(vec![
            Span::styled(format!("[{}] ", enabled), marker_style),
            Span::styled(key_display.clone(), key_style),
            Span::styled(": ", sep_style),
            Span::styled(value_display, value_style),
        ]));

        if editing_key {
            if let Some(inp) = app.header_key_inputs.get(i) {
                let prefix_len = 4; // "[x] "
                let cx = area.x + prefix_len + inp.cursor() as u16;
                let cy = area.y + i as u16;
                if cx < area.x + area.width && cy < area.y + area.height {
                    cursor_pos = Some((cx, cy));
                }
            }
        } else if editing_value {
            if let Some(inp) = app.header_value_inputs.get(i) {
                let prefix_len = 4 + key_display.len() as u16 + 2; // "[x] " + key + ": "
                let cx = area.x + prefix_len + inp.cursor() as u16;
                let cy = area.y + i as u16;
                if cx < area.x + area.width && cy < area.y + area.height {
                    cursor_pos = Some((cx, cy));
                }
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), area);

    if let Some((cx, cy)) = cursor_pos {
        frame.set_cursor_position((cx, cy));
    }
}

fn draw_body(frame: &mut Frame, app: &App, area: Rect) {
    let body_editing =
        app.input_mode == InputMode::Editing && app.edit_field == Some(EditField::BodyContent);

    let content = if body_editing {
        app.body_input.content().to_string()
    } else if let Some(req) = app.current_request() {
        match &req.body {
            Some(lazycurl_core::types::Body::Json { content }) => content.clone(),
            Some(lazycurl_core::types::Body::Text { content }) => content.clone(),
            _ => String::new(),
        }
    } else {
        String::new()
    };

    let style = if body_editing {
        Style::default().fg(Color::White).bg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };

    let display = if content.is_empty() && !body_editing {
        " Press Enter to edit body...".to_string()
    } else {
        format!(" {}", content)
    };

    frame.render_widget(Paragraph::new(display).style(style), area);

    if body_editing {
        let cursor_x = area.x + 1 + app.body_input.cursor() as u16;
        let cursor_y = area.y;
        if cursor_x < area.x + area.width {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

fn draw_auth(frame: &mut Frame, app: &App, area: Rect) {
    let Some(req) = app.current_request() else {
        return;
    };

    let text = match &req.auth {
        Some(lazycurl_core::types::Auth::Bearer { token }) => {
            format!(" Type: Bearer\n Token: {}", token)
        }
        Some(lazycurl_core::types::Auth::Basic { username, password }) => {
            format!(
                " Type: Basic\n Username: {}\n Password: {}",
                username, password
            )
        }
        Some(lazycurl_core::types::Auth::ApiKey {
            key,
            value,
            location,
        }) => format!(
            " Type: API Key\n Key: {}\n Value: {}\n In: {:?}",
            key, value, location
        ),
        Some(lazycurl_core::types::Auth::None) | None => {
            " No authentication configured.".to_string()
        }
    };

    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::White)),
        area,
    );
}

fn draw_params(frame: &mut Frame, app: &App, area: Rect) {
    let Some(req) = app.current_request() else {
        return;
    };

    if req.params.is_empty() {
        let text = Paragraph::new(" No query parameters. Press 'a' to add one.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(text, area);
        return;
    }

    let is_focused = app.active_pane == Pane::Request;
    let mut lines = Vec::new();
    let mut cursor_pos: Option<(u16, u16)> = None;

    for (i, param) in req.params.iter().enumerate() {
        let is_selected = i == app.param_cursor && is_focused;
        let enabled = if param.enabled { " " } else { "x" };

        let editing_key = app.input_mode == InputMode::Editing
            && app.edit_field == Some(EditField::ParamKey(i));
        let editing_value = app.input_mode == InputMode::Editing
            && app.edit_field == Some(EditField::ParamValue(i));

        let key_display = if editing_key {
            app.param_key_inputs
                .get(i)
                .map(|inp| inp.content().to_string())
                .unwrap_or_default()
        } else {
            param.key.clone()
        };

        let value_display = if editing_value {
            app.param_value_inputs
                .get(i)
                .map(|inp| inp.content().to_string())
                .unwrap_or_default()
        } else {
            param.value.clone()
        };

        let base_style = if !param.enabled {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default().fg(Color::White)
        };

        let key_style = if editing_key {
            Style::default().fg(Color::Yellow).bg(Color::DarkGray)
        } else if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let value_style = if editing_value {
            base_style.bg(Color::DarkGray)
        } else if is_selected {
            base_style.add_modifier(Modifier::REVERSED)
        } else {
            base_style
        };

        let marker_style = if is_selected {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let sep_style = if is_selected {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        lines.push(Line::from(vec![
            Span::styled(format!("[{}] ", enabled), marker_style),
            Span::styled(key_display.clone(), key_style),
            Span::styled("=", sep_style),
            Span::styled(value_display, value_style),
        ]));

        if editing_key {
            if let Some(inp) = app.param_key_inputs.get(i) {
                let prefix_len = 4; // "[x] "
                let cx = area.x + prefix_len + inp.cursor() as u16;
                let cy = area.y + i as u16;
                if cx < area.x + area.width && cy < area.y + area.height {
                    cursor_pos = Some((cx, cy));
                }
            }
        } else if editing_value {
            if let Some(inp) = app.param_value_inputs.get(i) {
                let prefix_len = 4 + key_display.len() as u16 + 1; // "[x] " + key + "="
                let cx = area.x + prefix_len + inp.cursor() as u16;
                let cy = area.y + i as u16;
                if cx < area.x + area.width && cy < area.y + area.height {
                    cursor_pos = Some((cx, cy));
                }
            }
        }
    }

    frame.render_widget(Paragraph::new(lines), area);

    if let Some((cx, cy)) = cursor_pos {
        frame.set_cursor_position((cx, cy));
    }
}

pub fn draw_method_picker(frame: &mut Frame, app: &App, request_area: Rect) {
    use lazycurl_core::types::Method;
    use ratatui::widgets::{Clear, List, ListItem};

    let methods = Method::ALL;
    let items: Vec<ListItem> = methods
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let style = if i == app.method_picker_cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(method_color(*m))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(method_color(*m))
            };
            ListItem::new(format!(" {} ", m)).style(style)
        })
        .collect();

    let width = 12u16;
    let height = methods.len() as u16 + 2; // +2 for border
                                           // Position dropdown below the method+URL bar (row 2 in the request pane inner area)
    let x = request_area.x + 1;
    let y = request_area.y + 3; // after name line + method/url line + border
    let area = Rect::new(
        x,
        y,
        width.min(request_area.width),
        height.min(request_area.height.saturating_sub(3)),
    );

    frame.render_widget(Clear, area);
    let list = List::new(items).block(
        Block::default()
            .title(" Method ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    frame.render_widget(list, area);
}

fn method_color(method: lazycurl_core::types::Method) -> Color {
    match method {
        lazycurl_core::types::Method::Get => Color::Green,
        lazycurl_core::types::Method::Post => Color::Yellow,
        lazycurl_core::types::Method::Put => Color::Blue,
        lazycurl_core::types::Method::Delete => Color::Red,
        lazycurl_core::types::Method::Patch => Color::Magenta,
        lazycurl_core::types::Method::Head => Color::Cyan,
        lazycurl_core::types::Method::Options => Color::Gray,
    }
}
