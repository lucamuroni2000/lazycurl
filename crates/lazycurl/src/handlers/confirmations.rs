use crate::app::{Action, App};

/// Handle collection/request delete confirmation.
pub fn handle_collection_delete(app: &mut App, action: &Action) {
    match action {
        Action::ConfirmYes => {
            app.confirm_delete = false;
            app.delete_selected_in_collections();
        }
        Action::Cancel => {
            app.confirm_delete = false;
            app.status_message = None;
        }
        _ => {} // Ignore unrecognized keys while confirmation is shown
    }
}

/// Handle variable delete confirmation.
pub fn handle_variable_delete(app: &mut App, action: &Action) {
    match action {
        Action::ConfirmYes => {
            app.var_confirm_delete = false;
            app.var_delete_message = None;
            app.var_delete();
        }
        Action::Cancel => {
            app.var_confirm_delete = false;
            app.var_delete_message = None;
        }
        _ => {}
    }
}

/// Handle environment delete confirmation.
pub fn handle_env_delete(app: &mut App, action: &Action) {
    match action {
        Action::ConfirmYes => {
            app.env_manager_execute_delete();
        }
        Action::Cancel => {
            app.env_manager_confirm_delete = None;
        }
        _ => {}
    }
}

/// Handle project delete confirmation.
pub fn handle_project_delete(app: &mut App, action: &Action) {
    match action {
        Action::ConfirmYes => {
            app.project_picker_execute_delete();
        }
        Action::Cancel => {
            app.project_picker_confirm_delete = None;
        }
        _ => {}
    }
}
