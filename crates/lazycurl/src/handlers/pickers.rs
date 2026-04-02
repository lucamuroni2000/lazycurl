use crate::app::{self, Action, App, EditField};
use crate::handlers::confirmations;
use lazycurl_core::config::config_dir;

/// Handle method picker actions.
pub fn handle_method_picker(app: &mut App, action: &Action) {
    match action {
        Action::Cancel => {
            app.show_method_picker = false;
        }
        Action::MoveUp => {
            if app.method_picker_cursor > 0 {
                app.method_picker_cursor -= 1;
            }
        }
        Action::MoveDown => {
            if app.method_picker_cursor + 1 < lazycurl_core::types::Method::ALL.len() {
                app.method_picker_cursor += 1;
            }
        }
        Action::Enter => {
            let method = lazycurl_core::types::Method::ALL[app.method_picker_cursor];
            app.select_method(method);
        }
        Action::Quit => app.should_quit = true,
        _ => {}
    }
}

/// Handle auth type picker actions.
pub fn handle_auth_picker(app: &mut App, action: &Action) {
    match action {
        Action::Cancel => {
            app.show_auth_picker = false;
        }
        Action::MoveUp => {
            if app.auth_picker_cursor > 0 {
                app.auth_picker_cursor -= 1;
            }
        }
        Action::MoveDown => {
            if app.auth_picker_cursor < app::AUTH_TYPE_LABELS.len() - 1 {
                app.auth_picker_cursor += 1;
            }
        }
        Action::Enter => {
            app.select_auth_type(app.auth_picker_cursor);
        }
        Action::Quit => {
            app.should_quit = true;
        }
        _ => {}
    }
}

/// Handle export picker actions.
pub fn handle_export_picker(app: &mut App, action: &Action) {
    match action {
        Action::Cancel => {
            app.show_export_picker = false;
        }
        Action::MoveUp => {
            if app.export_format_cursor > 0 {
                app.export_format_cursor -= 1;
            }
        }
        Action::MoveDown => {
            let max = app.export_formats().len().saturating_sub(1);
            if app.export_format_cursor < max {
                app.export_format_cursor += 1;
            }
        }
        Action::CyclePaneForward => {
            if app.export_collection_available {
                app.export_scope_is_collection = !app.export_scope_is_collection;
                app.export_format_cursor = 0;
            }
        }
        Action::Enter => {
            let format = app.selected_export_format();
            crate::execute_export(app, format);
            app.show_export_picker = false;
        }
        Action::Quit => app.should_quit = true,
        _ => {}
    }
}

/// Handle collection picker actions.
pub fn handle_collection_picker(app: &mut App, action: &Action) {
    match action {
        Action::Cancel => {
            app.show_collection_picker = false;
            app.status_message = None;
        }
        Action::MoveUp => {
            if app.picker_cursor > 0 {
                app.picker_cursor -= 1;
            }
        }
        Action::MoveDown => {
            if app.picker_cursor + 1 < app.collections().len() {
                app.picker_cursor += 1;
            }
        }
        Action::Enter => {
            let idx = app.picker_cursor;
            app.show_collection_picker = false;
            app.save_request_to_collection(idx);
        }
        Action::NewRequest => {
            // Create new collection and save there
            app.show_collection_picker = false;
            app.name_input.set_content("My Collection");
            app.start_editing(EditField::NewCollectionName);
            app.status_message = Some("Name your collection, then press Enter to save".to_string());
        }
        Action::Quit => app.should_quit = true,
        _ => {}
    }
}

/// Handle project picker actions.
pub fn handle_project_picker(app: &mut App, action: &Action) {
    // Delete confirmation state
    if app.project_picker_confirm_delete.is_some() {
        confirmations::handle_project_delete(app, action);
        return;
    }

    // Renaming state
    if app.project_picker_renaming {
        match action {
            Action::Enter => {
                app.project_picker_confirm_rename();
            }
            Action::Cancel => {
                app.project_picker_renaming = false;
            }
            Action::CharInput(c) => {
                app.project_picker_name_input.insert_char(*c);
            }
            Action::Backspace => {
                app.project_picker_name_input.delete_char_before();
            }
            Action::Delete => {
                app.project_picker_name_input.delete_char_after();
            }
            Action::CursorLeft => {
                app.project_picker_name_input.move_left();
            }
            Action::CursorRight => {
                app.project_picker_name_input.move_right();
            }
            Action::Home => {
                app.project_picker_name_input.move_home();
            }
            Action::End => {
                app.project_picker_name_input.move_end();
            }
            _ => {}
        }
        return;
    }

    // Normal modal state
    match action {
        Action::Cancel => {
            app.show_project_picker = false;
        }
        Action::MoveUp => {
            if app.project_picker_cursor > 0 {
                app.project_picker_cursor -= 1;
            }
        }
        Action::MoveDown => {
            if app.project_picker_cursor + 1 < app.all_projects.len() {
                app.project_picker_cursor += 1;
            }
        }
        Action::Enter => {
            if let Some((project, slug)) = app.all_projects.get(app.project_picker_cursor).cloned()
            {
                // Check if already open
                if let Some(idx) = app.open_projects.iter().position(|ws| ws.data.slug == slug) {
                    app.switch_project(idx);
                } else {
                    // Open the project
                    let path = config_dir().join("projects").join(&slug);
                    let mut ws = app::ProjectWorkspace::new(project, slug);
                    ws.data.collections =
                        lazycurl_core::collection::list_collections(&path.join("collections"))
                            .unwrap_or_default();
                    ws.data.environments =
                        lazycurl_core::environment::list_environments(&path.join("environments"))
                            .unwrap_or_default();
                    ws.data.restore_active_environment();
                    app.open_projects.push(ws);
                    let idx = app.open_projects.len() - 1;
                    app.switch_project(idx);
                }
                app.show_project_picker = false;
            }
        }
        Action::NewRequest => {
            // Create new project inline in the picker
            let project = lazycurl_core::types::Project {
                id: uuid::Uuid::new_v4(),
                name: "New Project".to_string(),
                active_environment: None,
            };
            let projects_dir = config_dir().join("projects");
            if let Ok(dir) = lazycurl_core::project::create_project(&projects_dir, &project) {
                let slug = dir.file_name().unwrap().to_string_lossy().to_string();
                // Add to all_projects list and position cursor
                app.all_projects.push((project, slug));
                app.project_picker_cursor = app.all_projects.len() - 1;
                // Start inline rename so user can name it
                app.project_picker_start_rename();
            }
        }
        Action::Rename => {
            if !app.all_projects.is_empty() {
                app.project_picker_start_rename();
            }
        }
        Action::CloseProject => {
            // Close project (remove from tab bar)
            if let Some((_, slug)) = app.all_projects.get(app.project_picker_cursor) {
                if let Some(idx) = app
                    .open_projects
                    .iter()
                    .position(|ws| ws.data.slug == *slug)
                {
                    app.close_project(idx);
                }
            }
        }
        Action::DeleteItem => {
            if !app.all_projects.is_empty() {
                app.project_picker_request_delete();
            }
        }
        Action::Quit => app.should_quit = true,
        _ => {}
    }
}

/// Handle environment manager actions.
pub fn handle_env_manager(app: &mut App, action: &Action) {
    // Delete confirmation state
    if app.env_manager_confirm_delete.is_some() {
        confirmations::handle_env_delete(app, action);
        return;
    }

    // Renaming state
    if app.env_manager_renaming.is_some() {
        match action {
            Action::Enter => {
                app.env_manager_confirm_rename();
            }
            Action::Cancel => {
                app.env_manager_renaming = None;
            }
            Action::CharInput(c) => {
                app.env_manager_name_input.insert_char(*c);
            }
            Action::Backspace => {
                app.env_manager_name_input.delete_char_before();
            }
            Action::Delete => {
                app.env_manager_name_input.delete_char_after();
            }
            Action::CursorLeft => {
                app.env_manager_name_input.move_left();
            }
            Action::CursorRight => {
                app.env_manager_name_input.move_right();
            }
            Action::Home => {
                app.env_manager_name_input.move_home();
            }
            Action::End => {
                app.env_manager_name_input.move_end();
            }
            _ => {}
        }
        return;
    }

    // Normal modal state
    let env_count = app
        .active_workspace()
        .map(|ws| ws.data.environments.len())
        .unwrap_or(0);

    match action {
        Action::Cancel => {
            // Return to variables overlay if it was open underneath
            app.show_env_manager = false;
        }
        Action::MoveUp => {
            if app.env_manager_cursor > 0 {
                app.env_manager_cursor -= 1;
            }
        }
        Action::MoveDown => {
            if env_count > 0 && app.env_manager_cursor + 1 < env_count {
                app.env_manager_cursor += 1;
            }
        }
        Action::Enter => {
            if env_count > 0 {
                app.env_manager_activate();
            }
        }
        Action::NewRequest => {
            app.env_manager_create();
        }
        Action::Rename => {
            if env_count > 0 {
                app.env_manager_start_rename();
            }
        }
        Action::DeleteItem => {
            if env_count > 0 {
                app.env_manager_request_delete();
            }
        }
        Action::Quit => app.should_quit = true,
        _ => {}
    }
}
