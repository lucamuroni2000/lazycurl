use crate::app::{self, Action, App};

pub fn handle(app: &mut App, action: &Action) {
    // Search editing mode
    if app.help_editing_search {
        match action {
            Action::Enter => {
                app.help_search = app.help_search_input.content().to_string();
                app.help_editing_search = false;
                app.input_mode = app::InputMode::Normal;
                app.help_cursor = 0;
            }
            Action::Cancel => {
                app.help_editing_search = false;
                app.input_mode = app::InputMode::Normal;
            }
            Action::CharInput(c) => app.help_search_input.insert_char(*c),
            Action::Backspace => {
                app.help_search_input.delete_char_before();
            }
            Action::Delete => {
                app.help_search_input.delete_char_after();
            }
            Action::CursorLeft => {
                app.help_search_input.move_left();
            }
            Action::CursorRight => {
                app.help_search_input.move_right();
            }
            Action::Home => {
                app.help_search_input.move_home();
            }
            Action::End => {
                app.help_search_input.move_end();
            }
            Action::Quit => app.should_quit = true,
            _ => {}
        }
        return;
    }

    // Normal help overlay mode
    match action {
        Action::Help | Action::Cancel => {
            app.show_help = false;
            app.help_search.clear();
            app.help_cursor = 0;
        }
        Action::MoveUp => {
            app.help_cursor = app.help_cursor.saturating_sub(1);
        }
        Action::MoveDown => {
            // Increments freely; draw function clamps to entry count
            app.help_cursor += 1;
        }
        Action::Search => {
            app.help_editing_search = true;
            app.help_search_input.set_content(&app.help_search);
            app.input_mode = app::InputMode::Editing;
        }
        Action::CharInput(c) => {
            // Clear search using the same key as log_viewer.clear_search
            if !app.help_search.is_empty() {
                if let Some(key) = app.config.keybindings.get("log_viewer.clear_search") {
                    if c.to_string() == *key {
                        app.help_search.clear();
                        app.help_cursor = 0;
                    }
                }
            }
        }
        Action::Quit => {
            app.should_quit = true;
        }
        _ => {}
    }
}
