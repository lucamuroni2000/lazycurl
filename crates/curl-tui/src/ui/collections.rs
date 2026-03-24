use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, Pane};

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
        let text = Paragraph::new(" No collections.\n Press Ctrl+N to create a request.")
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(text, inner);
        return;
    }

    let mut lines = Vec::new();
    for (col_idx, collection) in app.collections().iter().enumerate() {
        let is_selected_col =
            app.selected_collection() == Some(col_idx) && app.selected_request().is_none();

        let style = if is_selected_col && is_focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        };

        lines.push(Line::from(Span::styled(
            format!(
                " {} {}",
                if collection.requests.is_empty() {
                    " "
                } else {
                    ">"
                },
                collection.name
            ),
            style,
        )));

        // Show requests under the collection
        for (req_idx, req) in collection.requests.iter().enumerate() {
            let is_selected_req = app.selected_collection() == Some(col_idx)
                && app.selected_request() == Some(req_idx);

            let method_style = Style::default().fg(method_color(req.method));
            let name_style = if is_selected_req && is_focused {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::REVERSED)
            } else {
                Style::default().fg(Color::White)
            };

            lines.push(Line::from(vec![
                Span::raw("   "),
                Span::styled(format!("{:7}", req.method), method_style),
                Span::styled(&req.name, name_style),
            ]));
        }
    }

    // Handle scrolling
    let visible_height = inner.height as usize;
    let start = app.collection_scroll();
    let end = (start + visible_height).min(lines.len());
    let visible_lines: Vec<Line> = lines[start..end].to_vec();

    frame.render_widget(Paragraph::new(visible_lines), inner);
}

fn method_color(method: curl_tui_core::types::Method) -> Color {
    match method {
        curl_tui_core::types::Method::Get => Color::Green,
        curl_tui_core::types::Method::Post => Color::Yellow,
        curl_tui_core::types::Method::Put => Color::Blue,
        curl_tui_core::types::Method::Delete => Color::Red,
        curl_tui_core::types::Method::Patch => Color::Magenta,
        curl_tui_core::types::Method::Head => Color::Cyan,
        curl_tui_core::types::Method::Options => Color::Gray,
    }
}
