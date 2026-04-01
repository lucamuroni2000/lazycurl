use crate::app::{self, Action, App};

pub fn handle(app: &mut App, action: &Action) {
    // Filter editing mode
    if app.log_viewer_editing_filter {
        match action {
            Action::Enter => {
                app.log_viewer_filter = app.log_viewer_filter_input.content().to_string();
                app.log_viewer_editing_filter = false;
                app.input_mode = app::InputMode::Normal;
            }
            Action::Cancel => {
                app.log_viewer_editing_filter = false;
                app.input_mode = app::InputMode::Normal;
            }
            Action::CharInput(c) => app.log_viewer_filter_input.insert_char(*c),
            Action::Backspace => {
                app.log_viewer_filter_input.delete_char_before();
            }
            Action::Delete => {
                app.log_viewer_filter_input.delete_char_after();
            }
            Action::CursorLeft => {
                app.log_viewer_filter_input.move_left();
            }
            Action::CursorRight => {
                app.log_viewer_filter_input.move_right();
            }
            Action::Home => {
                app.log_viewer_filter_input.move_home();
            }
            Action::End => {
                app.log_viewer_filter_input.move_end();
            }
            Action::Quit => app.should_quit = true,
            _ => {}
        }
        return;
    }

    // Search editing mode
    if app.log_viewer_editing_search {
        match action {
            Action::Enter => {
                app.log_viewer_search = app.log_viewer_search_input.content().to_string();
                app.log_viewer_editing_search = false;
                app.input_mode = app::InputMode::Normal;
                // Jump to first matching entry
                if !app.log_viewer_search.is_empty() {
                    let search_lower = app.log_viewer_search.to_lowercase();
                    let filtered = app.filtered_log_entries();
                    if let Some(pos) = filtered
                        .iter()
                        .position(|e| e.request.url.to_lowercase().contains(&search_lower))
                    {
                        app.log_viewer_cursor = pos;
                    }
                }
            }
            Action::Cancel => {
                app.log_viewer_editing_search = false;
                app.input_mode = app::InputMode::Normal;
            }
            Action::CharInput(c) => app.log_viewer_search_input.insert_char(*c),
            Action::Backspace => {
                app.log_viewer_search_input.delete_char_before();
            }
            Action::Delete => {
                app.log_viewer_search_input.delete_char_after();
            }
            Action::CursorLeft => {
                app.log_viewer_search_input.move_left();
            }
            Action::CursorRight => {
                app.log_viewer_search_input.move_right();
            }
            Action::Home => {
                app.log_viewer_search_input.move_home();
            }
            Action::End => {
                app.log_viewer_search_input.move_end();
            }
            Action::Quit => app.should_quit = true,
            _ => {}
        }
        return;
    }

    // Normal log viewer mode
    let entry_count = app.filtered_log_entries().len();

    match action {
        Action::OpenLogViewer | Action::Cancel => {
            if app.log_viewer_detail_focused {
                app.log_viewer_detail_focused = false;
            } else if app.log_viewer_show_detail {
                app.log_viewer_show_detail = false;
            } else {
                app.show_log_viewer = false;
            }
        }
        Action::CyclePaneForward | Action::CyclePaneBackward => {
            if app.log_viewer_show_detail {
                app.log_viewer_detail_focused = !app.log_viewer_detail_focused;
            }
        }
        Action::MoveUp => {
            if app.log_viewer_detail_focused {
                if app.log_viewer_detail_scroll > 0 {
                    app.log_viewer_detail_scroll -= 1;
                }
            } else if app.log_viewer_cursor > 0 {
                app.log_viewer_cursor -= 1;
                app.log_viewer_detail_scroll = 0;
            }
        }
        Action::MoveDown => {
            if app.log_viewer_detail_focused {
                let total = app.log_viewer_detail_total_lines();
                if total > 0 && app.log_viewer_detail_scroll + 1 < total {
                    app.log_viewer_detail_scroll += 1;
                }
            } else if entry_count > 0 && app.log_viewer_cursor + 1 < entry_count {
                app.log_viewer_cursor += 1;
                app.log_viewer_detail_scroll = 0;
            }
        }
        Action::Enter => {
            if !app.log_viewer_show_detail {
                app.log_viewer_show_detail = true;
                app.log_viewer_detail_focused = true;
                app.log_viewer_detail_scroll = 0;
            } else {
                app.log_viewer_detail_focused = !app.log_viewer_detail_focused;
            }
        }
        Action::Search => {
            app.log_viewer_editing_search = true;
            app.log_viewer_search_input
                .set_content(&app.log_viewer_search);
            app.input_mode = app::InputMode::Editing;
        }
        Action::LogFilter => {
            app.log_viewer_editing_filter = true;
            app.log_viewer_filter_input
                .set_content(&app.log_viewer_filter);
            app.input_mode = app::InputMode::Editing;
        }
        Action::LogClearFilter => {
            app.log_viewer_filter.clear();
            app.log_viewer_cursor = 0;
        }
        Action::LogClearSearch => {
            app.log_viewer_search.clear();
        }
        Action::LogNextMatch => {
            if !app.log_viewer_search.is_empty() {
                let search_lower = app.log_viewer_search.to_lowercase();
                let filtered = app.filtered_log_entries();
                let start = app.log_viewer_cursor + 1;
                if let Some(pos) = filtered
                    .iter()
                    .skip(start)
                    .position(|e| e.request.url.to_lowercase().contains(&search_lower))
                {
                    app.log_viewer_cursor = start + pos;
                } else if let Some(pos) = filtered
                    .iter()
                    .position(|e| e.request.url.to_lowercase().contains(&search_lower))
                {
                    app.log_viewer_cursor = pos;
                }
            }
        }
        Action::LogPrevMatch => {
            if !app.log_viewer_search.is_empty() {
                let search_lower = app.log_viewer_search.to_lowercase();
                let filtered = app.filtered_log_entries();
                if app.log_viewer_cursor > 0 {
                    if let Some(pos) = filtered[..app.log_viewer_cursor]
                        .iter()
                        .rposition(|e| e.request.url.to_lowercase().contains(&search_lower))
                    {
                        app.log_viewer_cursor = pos;
                    } else if let Some(pos) = filtered
                        .iter()
                        .rposition(|e| e.request.url.to_lowercase().contains(&search_lower))
                    {
                        app.log_viewer_cursor = pos;
                    }
                } else if let Some(pos) = filtered
                    .iter()
                    .rposition(|e| e.request.url.to_lowercase().contains(&search_lower))
                {
                    app.log_viewer_cursor = pos;
                }
            }
        }
        Action::Rename => {
            // Re-send: load request into editor
            let filtered = app.filtered_log_entries();
            if let Some(entry) = filtered.get(app.log_viewer_cursor) {
                app.load_log_entry_into_editor(entry.clone());
                app.show_log_viewer = false;
            }
        }
        Action::Copy => {
            // Copy response body to clipboard
            let filtered = app.filtered_log_entries();
            if let Some(entry) = filtered.get(app.log_viewer_cursor) {
                let text = entry
                    .response
                    .as_ref()
                    .and_then(|r| r.body.as_deref())
                    .unwrap_or("[no response body]");
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    let _ = clipboard.set_text(text);
                    app.status_message = Some("Copied response body to clipboard".to_string());
                }
            }
        }
        Action::LogCopyPath => {
            let logs_path = lazycurl_core::logging::logs_dir();
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(logs_path.to_string_lossy().to_string());
                app.status_message = Some(format!("Copied: {}", logs_path.display()));
            }
        }
        Action::LogExport => {
            let filtered = app.filtered_log_entries();
            let now = chrono::Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
            let filename = format!("lazycurl-export-{}.jsonl", now);
            let path = std::env::current_dir().unwrap_or_default().join(&filename);
            if let Ok(mut file) = std::fs::File::create(&path) {
                use std::io::Write;
                for entry in &filtered {
                    if let Ok(line) = serde_json::to_string(entry) {
                        let _ = writeln!(file, "{}", line);
                    }
                }
                app.status_message = Some(format!("Exported to {}", path.display()));
                crate::open_in_file_explorer(&path);
            }
        }
        Action::Quit => {
            app.should_quit = true;
        }
        _ => {}
    }
}
