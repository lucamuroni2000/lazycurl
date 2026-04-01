use crate::app::{self, Action, App};
use lazycurl_core::config::config_dir;
use lazycurl_core::types::Auth;

pub async fn handle(app: &mut App, action: &Action) {
    match action {
        Action::Quit => app.should_quit = true,
        Action::Cancel => {
            if app.input_mode == app::InputMode::Editing {
                app.stop_editing();
            } else if app.show_help {
                app.show_help = false;
            }
        }
        Action::CyclePaneForward => {
            if app.input_mode == app::InputMode::Editing {
                match app.edit_field {
                    Some(app::EditField::HeaderKey(i)) => {
                        app.stop_editing();
                        app.start_editing(app::EditField::HeaderValue(i));
                        return;
                    }
                    Some(app::EditField::HeaderValue(_)) => {
                        app.stop_editing();
                        return;
                    }
                    Some(app::EditField::ParamKey(i)) => {
                        app.stop_editing();
                        app.start_editing(app::EditField::ParamValue(i));
                        return;
                    }
                    Some(app::EditField::ParamValue(_)) => {
                        app.stop_editing();
                        return;
                    }
                    _ => {
                        app.stop_editing();
                    }
                }
            }
            app.cycle_pane_forward();
        }
        Action::CyclePaneBackward => {
            if app.input_mode == app::InputMode::Editing {
                app.stop_editing();
            }
            app.cycle_pane_backward();
        }
        Action::FocusCollections => app.toggle_pane(0),
        Action::FocusRequest => app.toggle_pane(1),
        Action::FocusResponse => app.toggle_pane(2),
        Action::FocusUrl => {
            if app.active_pane == app::Pane::Request {
                app.start_editing(app::EditField::Url);
            }
        }
        Action::RevealSecrets => app.secrets_revealed = !app.secrets_revealed,
        Action::OpenLogViewer => {
            if !app.show_method_picker
                && !app.show_collection_picker
                && !app.show_project_picker
                && !app.show_env_manager
                && !app.show_variables
            {
                app.open_log_viewer();
            }
        }
        Action::Help => app.show_help = !app.show_help,
        Action::OpenVariables => {
            app.show_variables = true;
            app.var_cursor = 0;
            app.var_editing = None;
            if let Some(ws) = app.active_workspace_mut() {
                ws.data.var_collection_idx =
                    ws.data
                        .selected_collection
                        .or(if ws.data.collections.is_empty() {
                            None
                        } else {
                            Some(0)
                        });
                ws.data.var_environment_idx =
                    ws.data
                        .active_environment
                        .or(if ws.data.environments.is_empty() {
                            None
                        } else {
                            Some(0)
                        });
            }
        }
        Action::SendRequest => {
            if app.input_mode == app::InputMode::Editing {
                app.stop_editing();
            }
            // If on Auth tab with OAuth 2.0, trigger the OAuth flow instead
            if app.request_tab() == app::RequestTab::Auth {
                if let Some(req) = app.current_request() {
                    if matches!(req.auth, Some(Auth::OAuth2 { .. })) {
                        app.trigger_oauth2_flow().await;
                        return;
                    }
                }
            }
            app.send_request().await;
        }
        Action::SaveRequest => app.save_current_request(),
        Action::NewRequest => {
            if app.active_pane == app::Pane::Collections {
                app.create_new_collection();
            } else {
                app.new_request();
                app.load_request_into_inputs();
            }
        }
        Action::SwitchEnvironment => app.cycle_environment(),
        Action::ManageEnvironments => app.open_env_manager(),
        Action::OpenExportPicker => {
            if !app.show_method_picker
                && !app.show_collection_picker
                && !app.show_project_picker
                && !app.show_env_manager
                && !app.show_variables
                && !app.show_log_viewer
            {
                app.open_export_picker();
            }
        }
        // Navigation actions (Normal mode)
        Action::MoveUp => app.handle_move_up(),
        Action::MoveDown => app.handle_move_down(),
        Action::Enter => {
            if app.input_mode == app::InputMode::Editing {
                app.stop_editing();
            } else {
                app.handle_enter();
            }
        }
        Action::NextTab => match app.active_pane {
            app::Pane::Request => app.next_request_tab(),
            app::Pane::Response => app.next_response_tab(),
            _ => {}
        },
        Action::PrevTab => match app.active_pane {
            app::Pane::Request => app.prev_request_tab(),
            app::Pane::Response => app.prev_response_tab(),
            _ => {}
        },
        Action::AddItem => {
            if app.active_pane == app::Pane::Request {
                match app.request_tab() {
                    app::RequestTab::Headers => app.add_header(),
                    app::RequestTab::Params => app.add_param(),
                    _ => {}
                }
            }
        }
        Action::NextProject => app.next_project(),
        Action::PrevProject => app.prev_project(),
        Action::OpenProjectPicker => {
            // Refresh the project list from disk
            let projects_dir = config_dir().join("projects");
            app.all_projects = lazycurl_core::project::list_projects(&projects_dir)
                .unwrap_or_default()
                .into_iter()
                .map(|(p, path)| {
                    let slug = path.file_name().unwrap().to_string_lossy().to_string();
                    (p, slug)
                })
                .collect();
            app.project_picker_cursor = 0;
            app.show_project_picker = true;
        }
        Action::DeleteItem => {
            if app.active_pane == app::Pane::Collections {
                app.request_collection_delete();
            } else if app.active_pane == app::Pane::Request {
                match app.request_tab() {
                    app::RequestTab::Headers => app.delete_header(),
                    app::RequestTab::Params => app.delete_param(),
                    _ => {}
                }
            }
        }
        Action::CycleMethod => {
            if app.active_pane == app::Pane::Request {
                app.open_method_picker();
            }
        }
        Action::ToggleEnabled => {
            if app.active_pane == app::Pane::Request {
                match app.request_tab() {
                    app::RequestTab::Headers => app.toggle_header_enabled(),
                    app::RequestTab::Params => app.toggle_param_enabled(),
                    _ => {}
                }
            }
        }
        Action::Copy => {
            if app.active_pane == app::Pane::Response
                && app.input_mode == app::InputMode::Normal
            {
                if let Some(resp) = app.last_response() {
                    let text = if resp.body.is_empty() {
                        "[no response body]".to_string()
                    } else {
                        resp.body.clone()
                    };
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(&text);
                        app.status_message =
                            Some("Copied response body to clipboard".to_string());
                    }
                }
            }
        }
        Action::Rename => app.handle_rename(),
        Action::ChangeAuthType => {
            if app.active_pane == app::Pane::Request
                && app.request_tab() == app::RequestTab::Auth
                && app.input_mode == app::InputMode::Normal
            {
                app.open_auth_picker();
            }
        }
        // Editing actions
        Action::CharInput(c) => {
            if let Some(input) = app.active_text_input() {
                input.insert_char(*c);
            }
        }
        Action::Backspace => {
            if let Some(input) = app.active_text_input() {
                input.delete_char_before();
            }
        }
        Action::Delete => {
            if let Some(input) = app.active_text_input() {
                input.delete_char_after();
            }
        }
        Action::CursorLeft => {
            if let Some(input) = app.active_text_input() {
                input.move_left();
            }
        }
        Action::CursorRight => {
            if let Some(input) = app.active_text_input() {
                input.move_right();
            }
        }
        Action::Home => {
            if let Some(input) = app.active_text_input() {
                input.move_home();
            }
        }
        Action::End => {
            if let Some(input) = app.active_text_input() {
                input.move_end();
            }
        }
        _ => {}
    }
}
