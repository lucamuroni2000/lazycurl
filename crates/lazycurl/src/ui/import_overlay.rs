use std::collections::HashMap;

use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use super::{key_for, key_pair_for};
use crate::app::{App, ImportStep};

pub fn draw(frame: &mut Frame, app: &App, kb: &HashMap<String, String>) {
    match &app.import_step {
        ImportStep::FormatSelect => draw_format_select(frame, app, kb),
        ImportStep::Input => draw_input(frame, app, kb),
        ImportStep::Result => draw_result(frame, app, kb),
    }
}

fn draw_format_select(frame: &mut Frame, app: &App, kb: &HashMap<String, String>) {
    let formats = lazycurl_core::import::ImportFormat::all();
    let height = (formats.len() as u16 + 5).min(20);
    let area = centered_rect(50, height, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Import ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (i, format) in formats.iter().enumerate() {
        let is_selected = i == app.import_format_cursor;
        let marker = if is_selected { ">" } else { " " };
        let name_style = if is_selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default().fg(Color::White)
        };
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", marker), Style::default().fg(Color::Green)),
            Span::styled(format.label(), name_style),
        ]));
    }

    lines.push(Line::from(""));

    let hint_style = Style::default().fg(Color::DarkGray);
    let key_style = Style::default().fg(Color::Yellow);
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled(key_pair_for(kb, "move_up", "move_down"), key_style),
        Span::styled(":select ", hint_style),
        Span::styled(key_for(kb, "enter"), key_style),
        Span::styled(":next ", hint_style),
        Span::styled(key_for(kb, "cancel"), key_style),
        Span::styled(":cancel", hint_style),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);
}

fn draw_input(frame: &mut Frame, app: &App, kb: &HashMap<String, String>) {
    let formats = lazycurl_core::import::ImportFormat::all();
    let format = formats[app.import_format_cursor];
    let title = format!(" Import {} ", format.label());
    let is_curl = app.import_format_cursor == 0;

    let height = 8u16;
    let area = centered_rect(60, height, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(title.as_str())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    let label = if is_curl {
        " Paste or type curl command:"
    } else {
        " File path:"
    };
    lines.push(Line::from(Span::styled(
        label,
        Style::default().fg(Color::DarkGray),
    )));

    // Text input content
    let content = app.import_text_input.content();
    let display = if content.is_empty() {
        if is_curl {
            "curl ..."
        } else {
            "/path/to/file.json"
        }
    } else {
        content
    };
    let content_style = if content.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled(display, content_style),
    ]));
    lines.push(Line::from(""));

    let hint_style = Style::default().fg(Color::DarkGray);
    let key_style = Style::default().fg(Color::Yellow);
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled(key_for(kb, "enter"), key_style),
        Span::styled(":import ", hint_style),
        Span::styled(key_for(kb, "cancel"), key_style),
        Span::styled(":back", hint_style),
    ]));

    frame.render_widget(Paragraph::new(lines), inner);

    // Show cursor position
    if !content.is_empty() {
        let cursor_x = inner.x + 1 + app.import_text_input.cursor() as u16;
        let cursor_y = inner.y + 1;
        frame.set_cursor_position((cursor_x.min(inner.right() - 1), cursor_y));
    }
}

fn draw_result(frame: &mut Frame, app: &App, kb: &HashMap<String, String>) {
    let result = match &app.import_result {
        Some(r) => r,
        None => return,
    };

    let warning_lines = result.warnings.len() as u16;
    let height = if result.success {
        7 + warning_lines + if warning_lines > 0 { 2 } else { 0 }
    } else {
        7
    };
    let area = centered_rect(60, height.min(25), frame.area());
    frame.render_widget(Clear, area);

    let title = if result.success {
        " Import Result "
    } else {
        " Import Failed "
    };
    let border_color = if result.success {
        Color::Green
    } else {
        Color::Red
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    if result.success {
        lines.push(Line::from(vec![
            Span::styled(" + ", Style::default().fg(Color::Green)),
            Span::styled(
                format!("Imported \"{}\"", result.collection_name),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            format!(
                "   {} requests, {} variables",
                result.request_count, result.variable_count
            ),
            Style::default().fg(Color::DarkGray),
        )));

        if !result.warnings.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!(" ! {} warnings:", result.warnings.len()),
                Style::default().fg(Color::Yellow),
            )));
            for w in &result.warnings {
                lines.push(Line::from(Span::styled(
                    format!("   - {}", w),
                    Style::default().fg(Color::Yellow),
                )));
            }
        }
    } else if let Some(err) = &result.error {
        lines.push(Line::from(vec![
            Span::styled(" x ", Style::default().fg(Color::Red)),
            Span::styled(err.clone(), Style::default().fg(Color::White)),
        ]));
    }

    lines.push(Line::from(""));
    let hint_style = Style::default().fg(Color::DarkGray);
    let key_style = Style::default().fg(Color::Yellow);
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled(key_for(kb, "enter"), key_style),
        Span::styled(":done ", hint_style),
        Span::styled(key_for(kb, "cancel"), key_style),
        Span::styled(":close", hint_style),
    ]));

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
