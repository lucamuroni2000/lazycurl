use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, EditField, InputMode, Pane, RequestTab};

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
            | Some(EditField::NewProjectName)
    );

    if is_name_edit && app.input_mode == InputMode::Editing {
        let label = match app.edit_field {
            Some(EditField::NewCollectionName) => "Collection name",
            Some(EditField::CollectionName(_)) => "Rename collection",
            Some(EditField::RequestName) => "Request name",
            Some(EditField::NewProjectName) => "Project name",
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

    let hint_style = Style::default().fg(Color::DarkGray);
    let key_style = Style::default().fg(Color::Yellow);

    let mut hints: Vec<Span> = Vec::new();

    // Log viewer hints (checked early, before other overlays)
    if app.show_log_viewer {
        if app.log_viewer_editing_filter {
            hints.push(Span::styled(" Enter", key_style));
            hints.push(Span::styled(":apply ", hint_style));
            hints.push(Span::styled("Esc", key_style));
            hints.push(Span::styled(":cancel", hint_style));
        } else if app.log_viewer_editing_search {
            hints.push(Span::styled(" Enter", key_style));
            hints.push(Span::styled(":search ", hint_style));
            hints.push(Span::styled("Esc", key_style));
            hints.push(Span::styled(":cancel", hint_style));
        } else {
            hints.push(Span::styled(" j/k", key_style));
            hints.push(Span::styled(":nav ", hint_style));
            hints.push(Span::styled("Enter", key_style));
            hints.push(Span::styled(":detail ", hint_style));
            hints.push(Span::styled("f", key_style));
            hints.push(Span::styled(":filter ", hint_style));
            hints.push(Span::styled("/", key_style));
            hints.push(Span::styled(":search ", hint_style));
            hints.push(Span::styled("n/N", key_style));
            hints.push(Span::styled(":next/prev ", hint_style));
            hints.push(Span::styled("c/C", key_style));
            hints.push(Span::styled(":clear ", hint_style));
            hints.push(Span::styled("r", key_style));
            hints.push(Span::styled(":re-send ", hint_style));
            hints.push(Span::styled("y", key_style));
            hints.push(Span::styled(":body ", hint_style));
            hints.push(Span::styled("e", key_style));
            hints.push(Span::styled(":export ", hint_style));
            hints.push(Span::styled("Esc", key_style));
            hints.push(Span::styled(":close", hint_style));
        }

        let mut line_spans = vec![mode_indicator, Span::raw(" "), status_msg];
        line_spans.extend(hints);
        let line = Line::from(line_spans);

        let status = Paragraph::new(line).style(Style::default().bg(Color::Black));
        frame.render_widget(status, area);
        return;
    }

    if app.input_mode == InputMode::Editing {
        // Editing mode — show editing-specific hints
        hints.push(Span::styled(" Esc", key_style));
        hints.push(Span::styled(":done ", hint_style));
        hints.push(Span::styled("F5", key_style));
        hints.push(Span::styled(":send ", hint_style));
        hints.push(Span::styled("Tab", key_style));
        hints.push(Span::styled(":pane ", hint_style));
    } else {
        // Normal mode — context-sensitive hints

        // Always available
        hints.push(Span::styled(" Tab", key_style));
        hints.push(Span::styled(":pane ", hint_style));

        // Pane-specific hints
        match app.active_pane {
            Pane::Collections => {
                hints.push(Span::styled("Up/Down", key_style));
                hints.push(Span::styled(":navigate ", hint_style));
                hints.push(Span::styled("Enter", key_style));
                hints.push(Span::styled(":load ", hint_style));
                hints.push(Span::styled("Ctrl+N", key_style));
                hints.push(Span::styled(":new collection ", hint_style));
                hints.push(Span::styled("r", key_style));
                hints.push(Span::styled(":rename ", hint_style));
                hints.push(Span::styled("d", key_style));
                hints.push(Span::styled(":delete ", hint_style));
            }
            Pane::Request => {
                hints.push(Span::styled("Left/Right", key_style));
                hints.push(Span::styled(":tab ", hint_style));
                hints.push(Span::styled("Enter", key_style));
                hints.push(Span::styled(":edit ", hint_style));

                // Tab-specific hints
                match app.request_tab() {
                    RequestTab::Headers => {
                        hints.push(Span::styled("a", key_style));
                        hints.push(Span::styled(":add header ", hint_style));
                    }
                    RequestTab::Body => {}
                    RequestTab::Auth => {}
                    RequestTab::Params => {
                        hints.push(Span::styled("a", key_style));
                        hints.push(Span::styled(":add param ", hint_style));
                    }
                }

                hints.push(Span::styled("m", key_style));
                hints.push(Span::styled(":method ", hint_style));
                hints.push(Span::styled("r", key_style));
                hints.push(Span::styled(":rename ", hint_style));
                hints.push(Span::styled("Ctrl+N", key_style));
                hints.push(Span::styled(":new request ", hint_style));
            }
            Pane::Response => {
                hints.push(Span::styled("Left/Right", key_style));
                hints.push(Span::styled(":tab ", hint_style));
                hints.push(Span::styled("Up/Down", key_style));
                hints.push(Span::styled(":scroll ", hint_style));
            }
        }

        // Separator
        hints.push(Span::styled("| ", hint_style));

        // Global actions (always available)
        hints.push(Span::styled("F6", key_style));
        hints.push(Span::styled(":project ", hint_style));
        hints.push(Span::styled("Ctrl+O", key_style));
        hints.push(Span::styled(":projects ", hint_style));
        hints.push(Span::styled("F5", key_style));
        hints.push(Span::styled(":send ", hint_style));
        hints.push(Span::styled("Ctrl+S", key_style));
        hints.push(Span::styled(":save ", hint_style));
        hints.push(Span::styled("v", key_style));
        hints.push(Span::styled(":vars ", hint_style));
        hints.push(Span::styled("?", key_style));
        hints.push(Span::styled(":help ", hint_style));
        hints.push(Span::styled("Ctrl+Q", key_style));
        hints.push(Span::styled(":quit", hint_style));
    }

    let mut line_spans = vec![mode_indicator, Span::raw(" "), status_msg];
    line_spans.extend(hints);
    let line = Line::from(line_spans);

    let status = Paragraph::new(line).style(Style::default().bg(Color::Black));
    frame.render_widget(status, area);
}

fn mode_indicator_len(mode: InputMode) -> u16 {
    match mode {
        InputMode::Normal => 8,  // " NORMAL "
        InputMode::Editing => 9, // " EDITING "
    }
}
