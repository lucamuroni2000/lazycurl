use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{build_sidebar_items, resolve_collection_path, App, Pane, SidebarItem};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.active_pane == Pane::Collections;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(" Collections ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.collections().is_empty() {
        let text = Paragraph::new(" No collections.\n Press n to create a request.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(text, inner);
        return;
    }

    let ws = match app.active_workspace() {
        Some(ws) => ws,
        None => return,
    };

    let sidebar_items = build_sidebar_items(&ws.data.collections, &ws.expanded_folders);
    let selected_idx = ws.selected_sidebar_index;

    let mut lines = Vec::new();
    for (flat_idx, item) in sidebar_items.iter().enumerate() {
        let is_selected = flat_idx == selected_idx && is_focused;

        match item {
            SidebarItem::Collection { path } => {
                let depth = path.len() - 1;
                let indent = "  ".repeat(depth);
                let col = match resolve_collection_path(&ws.data.collections, path) {
                    Some(c) => c,
                    None => continue,
                };
                let is_expanded = ws.expanded_folders.contains(path);
                let arrow = if is_expanded { "\u{25BC}" } else { "\u{25B6}" };
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                };
                lines.push(Line::from(Span::styled(
                    format!("{} {} {}", indent, arrow, col.name),
                    style,
                )));
            }
            SidebarItem::Request { path, request_idx } => {
                let depth = path.len();
                let indent = "  ".repeat(depth);
                let col = match resolve_collection_path(&ws.data.collections, path) {
                    Some(c) => c,
                    None => continue,
                };
                let req = match col.requests.get(*request_idx) {
                    Some(r) => r,
                    None => continue,
                };
                let method_style = Style::default().fg(method_color(req.method));
                let name_style = if is_selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(Color::White)
                };
                lines.push(Line::from(vec![
                    Span::raw(format!("{} ", indent)),
                    Span::styled(format!("{:7}", req.method), method_style),
                    Span::styled(&req.name, name_style),
                ]));
            }
        }
    }

    // Handle scrolling
    let visible_height = inner.height as usize;
    let start = app.collection_scroll();
    let end = (start + visible_height).min(lines.len());
    let visible_lines: Vec<Line> = lines[start..end].to_vec();

    frame.render_widget(Paragraph::new(visible_lines), inner);
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
