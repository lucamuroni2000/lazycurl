use crate::app::{self, Action, App};
use crate::handlers::confirmations;

pub fn handle(app: &mut App, action: &Action) {
    // Variable delete confirmation
    if app.var_confirm_delete {
        confirmations::handle_variable_delete(app, action);
        return;
    }

    match action {
        Action::Cancel => {
            if app.input_mode == app::InputMode::Editing {
                app.var_editing = None;
                app.input_mode = app::InputMode::Normal;
            } else {
                app.show_variables = false;
            }
        }
        Action::MoveUp => app.var_move_up(),
        Action::MoveDown => app.var_move_down(),
        Action::NextTab | Action::CyclePaneForward => app.var_next_tier(),
        Action::PrevTab | Action::CyclePaneBackward => app.var_prev_tier(),
        Action::Enter => {
            if app.input_mode == app::InputMode::Editing {
                // If editing key, move to value; if editing value, confirm
                if app.var_editing == Some(app::VarEditTarget::Key) {
                    app.var_stop_editing();
                    app.var_start_edit_value();
                } else {
                    app.var_stop_editing();
                }
            } else {
                // Start editing value of selected variable
                app.var_start_edit_value();
            }
        }
        Action::Rename => {
            // Edit key of selected variable
            if app.input_mode != app::InputMode::Editing {
                app.var_start_edit_key();
            }
        }
        Action::AddItem => {
            if app.input_mode != app::InputMode::Editing {
                // If on Environment/Collection tier with nothing selected, create one first
                let needs_container = match app.var_tier {
                    app::VarTier::Environment => app.active_environment().is_none(),
                    app::VarTier::Collection => app.selected_collection().is_none(),
                    app::VarTier::Global => false,
                };
                if needs_container {
                    match app.var_tier {
                        app::VarTier::Environment => {
                            app.status_message = Some(
                                "No environment selected. Press Ctrl+Shift+E to manage environments."
                                    .to_string(),
                            );
                        }
                        app::VarTier::Collection => {
                            app.status_message =
                                Some("Select or create a collection first (Ctrl+S)".to_string());
                        }
                        _ => {}
                    }
                } else {
                    app.var_add();
                }
            }
        }
        Action::ManageEnvironments => {
            // Open env manager on top of variables overlay
            app.open_env_manager();
        }
        Action::SwitchEnvironment => {
            // Cycle environments even while overlay is open
            app.cycle_environment();
            // Sync var_environment_idx to follow the active env
            if let Some(ws) = app.active_workspace_mut() {
                ws.data.var_environment_idx = ws.data.active_environment;
            }
        }
        Action::DeleteItem => {
            if app.input_mode != app::InputMode::Editing {
                app.var_request_delete();
            }
        }
        Action::ToggleEnabled => {
            if app.input_mode != app::InputMode::Editing {
                app.var_toggle_secret();
            }
        }
        Action::RevealSecrets => {
            app.secrets_revealed = !app.secrets_revealed;
        }
        Action::CycleContainerBackward => {
            if app.input_mode != app::InputMode::Editing && app.var_tier == app::VarTier::Collection
            {
                app.var_cycle_container_backward();
            }
        }
        Action::CycleContainerForward => {
            if app.input_mode != app::InputMode::Editing && app.var_tier == app::VarTier::Collection
            {
                app.var_cycle_container_forward();
            }
        }
        // Text editing actions for variable inputs
        Action::CharInput(c) => {
            if let Some(input) = app.var_active_input() {
                input.insert_char(*c);
            }
        }
        Action::Backspace => {
            if let Some(input) = app.var_active_input() {
                input.delete_char_before();
            }
        }
        Action::Delete => {
            if let Some(input) = app.var_active_input() {
                input.delete_char_after();
            }
        }
        Action::CursorLeft => {
            if let Some(input) = app.var_active_input() {
                input.move_left();
            }
        }
        Action::CursorRight => {
            if let Some(input) = app.var_active_input() {
                input.move_right();
            }
        }
        Action::Home => {
            if let Some(input) = app.var_active_input() {
                input.move_home();
            }
        }
        Action::End => {
            if let Some(input) = app.var_active_input() {
                input.move_end();
            }
        }
        Action::Quit => app.should_quit = true,
        _ => {}
    }
}
