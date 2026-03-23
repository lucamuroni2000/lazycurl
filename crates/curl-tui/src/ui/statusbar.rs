use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, EditField, InputMode};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let mode_indicator = match app.input_mode {
        InputMode::Normal => Span::styled(
            " NORMAL ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        InputMode::Editing => Span::styled(
            " EDITING ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    };

    // If editing a name field (collection or request name), show it prominently in the status bar
    let is_name_edit = matches!(
        app.edit_field,
        Some(EditField::NewCollectionName)
            | Some(EditField::CollectionName(_))
            | Some(EditField::RequestName)
    );

    if is_name_edit && app.input_mode == InputMode::Editing {
        let label = match app.edit_field {
            Some(EditField::NewCollectionName) => "Collection name",
            Some(EditField::CollectionName(_)) => "Rename collection",
            Some(EditField::RequestName) => "Request name",
            _ => "Name",
        };

        let line = Line::from(vec![
            mode_indicator,
            Span::raw(" "),
            Span::styled(format!(" {}: ", label), Style::default().fg(Color::Yellow)),
            Span::styled(
                app.name_input.content().to_string(),
                Style::default().fg(Color::White).bg(Color::DarkGray),
            ),
            Span::styled(
                "  Enter:confirm  Esc:cancel ",
                Style::default().fg(Color::DarkGray),
            ),
        ]);

        let status = Paragraph::new(line).style(Style::default().bg(Color::Black));
        frame.render_widget(status, area);

        // Show cursor in the status bar for name editing
        let cursor_x = area.x
            + mode_indicator_len(app.input_mode)
            + 1
            + label.len() as u16
            + 3
            + app.name_input.cursor() as u16;
        if cursor_x < area.x + area.width {
            frame.set_cursor_position((cursor_x, area.y));
        }
        return;
    }

    let status_msg = if let Some(msg) = &app.status_message {
        Span::styled(format!(" {} ", msg), Style::default().fg(Color::White))
    } else {
        Span::raw("")
    };

    let hints = match app.input_mode {
        InputMode::Normal => Span::styled(
            " Tab:pane  Enter:edit  r:rename  ?:help  Ctrl+Q:quit ",
            Style::default().fg(Color::DarkGray),
        ),
        InputMode::Editing => Span::styled(
            " Esc:done  Ctrl+Enter:send  Tab:next pane ",
            Style::default().fg(Color::DarkGray),
        ),
    };

    let line = Line::from(vec![mode_indicator, Span::raw(" "), status_msg, hints]);

    let status = Paragraph::new(line).style(Style::default().bg(Color::Black));
    frame.render_widget(status, area);
}

fn mode_indicator_len(mode: InputMode) -> u16 {
    match mode {
        InputMode::Normal => 8,  // " NORMAL "
        InputMode::Editing => 9, // " EDITING "
    }
}
