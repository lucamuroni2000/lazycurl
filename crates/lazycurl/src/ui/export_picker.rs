use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let formats = app.export_formats();
    let height = (formats.len() as u16 + 7).min(20);
    let area = centered_rect(50, height, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Export ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Scope toggle bar
    let scope_request_style = if !app.export_scope_is_collection {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let scope_collection_style = if app.export_scope_is_collection {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut scope_spans = vec![
        Span::raw(" "),
        Span::styled("Current Request", scope_request_style),
    ];

    if app.export_collection_available {
        let collection_name = app
            .active_workspace()
            .and_then(|ws| {
                ws.data
                    .selected_collection
                    .and_then(|idx| ws.data.collections.get(idx))
                    .map(|c| c.name.clone())
            })
            .unwrap_or_else(|| "Collection".to_string());
        scope_spans.push(Span::raw("  "));
        scope_spans.push(Span::styled(collection_name, scope_collection_style));
    }

    lines.push(Line::from(scope_spans));
    lines.push(Line::from(""));

    // Format list
    for (i, format) in formats.iter().enumerate() {
        let is_selected = i == app.export_format_cursor;
        let marker = if is_selected { ">" } else { " " };
        let name_style = if is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", marker), Style::default().fg(Color::Cyan)),
            Span::styled(format.label(), name_style),
        ]));
    }

    lines.push(Line::from(""));

    // Hints
    let hint_style = Style::default().fg(Color::DarkGray);
    let key_style = Style::default().fg(Color::Yellow);
    let mut hint_spans = vec![Span::raw(" ")];
    if app.export_collection_available {
        hint_spans.push(Span::styled("Tab", key_style));
        hint_spans.push(Span::styled(":scope ", hint_style));
    }
    hint_spans.push(Span::styled("j/k", key_style));
    hint_spans.push(Span::styled(":select ", hint_style));
    hint_spans.push(Span::styled("Enter", key_style));
    hint_spans.push(Span::styled(":export ", hint_style));
    hint_spans.push(Span::styled("Esc", key_style));
    hint_spans.push(Span::styled(":cancel", hint_style));
    lines.push(Line::from(hint_spans));

    frame.render_widget(Paragraph::new(lines), inner);
}

fn centered_rect(percent_x: u16, lines: u16, area: Rect) -> Rect {
    let height = lines.min(area.height);
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0]);
    horizontal[0]
}
