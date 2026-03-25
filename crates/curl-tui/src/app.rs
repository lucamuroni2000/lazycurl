use curl_tui_core::command::CurlCommandBuilder;
use curl_tui_core::config::{config_dir, AppConfig};
use curl_tui_core::history::append_entry_dual;
use curl_tui_core::types::{
    Auth, Body, Collection, CurlResponse, Environment, HistoryEntry, Method, Request,
};
use curl_tui_core::variable::FileVariableResolver;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    /// Keybindings are active — keypresses map to actions
    Normal,
    /// Text editing — keypresses go to the focused text field
    Editing,
}

/// Identifies which text field is currently being edited
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditField {
    Url,
    HeaderKey(usize),
    #[allow(dead_code)]
    HeaderValue(usize),
    ParamKey(usize),
    #[allow(dead_code)]
    ParamValue(usize),
    BodyContent,
    RequestName,
    CollectionName(usize),
    EnvironmentName(usize),
    /// Editing name for a new collection being created (not yet in the list)
    NewCollectionName,
    /// Editing name for a new project being created
    NewProjectName,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VarTier {
    Global,
    Environment,
    Collection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VarEditTarget {
    Key,
    Value,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Pane {
    Collections,
    Request,
    Response,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RequestTab {
    Headers,
    Body,
    Auth,
    Params,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResponseTab {
    Body,
    Headers,
    Timing,
}

#[derive(Clone)]
pub enum Action {
    // Existing actions
    Quit,
    Cancel,
    CyclePaneForward,
    CyclePaneBackward,
    SendRequest,
    SaveRequest,
    SwitchEnvironment,
    ManageEnvironments,
    NewRequest,
    CopyCurl,
    ToggleCollections,
    ToggleRequest,
    ToggleResponse,
    RevealSecrets,
    Help,
    Search,
    // New actions for interactive features
    MoveUp,
    MoveDown,
    Enter,
    NextTab,
    PrevTab,
    DeleteItem,
    AddItem,
    Rename,
    OpenVariables,
    ToggleSecretFlag,
    CharInput(char),
    Backspace,
    Delete,
    CursorLeft,
    CursorRight,
    Home,
    End,
    // Project actions
    NextProject,
    PrevProject,
    OpenProjectPicker,
    #[allow(dead_code)]
    CloseProject,
    None,
}

/// Per-project workspace — wraps core data with TUI-specific fields.
pub struct ProjectWorkspace {
    pub data: curl_tui_core::types::ProjectWorkspaceData,
    pub request_tab: RequestTab,
    pub response_tab: ResponseTab,
    pub collection_scroll: usize,
    pub response_scroll: usize,
}

impl ProjectWorkspace {
    pub fn new(project: curl_tui_core::types::Project, slug: String) -> Self {
        Self {
            data: curl_tui_core::types::ProjectWorkspaceData::new(project, slug),
            request_tab: RequestTab::Headers,
            response_tab: ResponseTab::Body,
            collection_scroll: 0,
            response_scroll: 0,
        }
    }
}

pub struct App {
    pub config: AppConfig,
    // Project management
    pub open_projects: Vec<ProjectWorkspace>,
    pub active_project_idx: Option<usize>,
    pub project_tab_scroll: usize,
    // UI state (global)
    pub active_pane: Pane,
    pub pane_visible: [bool; 3], // [collections, request, response]
    pub should_quit: bool,
    pub show_help: bool,
    pub secrets_revealed: bool,
    pub status_message: Option<String>,
    pub input_mode: InputMode,
    pub edit_field: Option<EditField>,
    // Text inputs (shared)
    pub url_input: crate::text_input::TextInput,
    pub body_input: crate::text_input::TextInput,
    pub header_key_inputs: Vec<crate::text_input::TextInput>,
    pub header_value_inputs: Vec<crate::text_input::TextInput>,
    pub param_key_inputs: Vec<crate::text_input::TextInput>,
    pub param_value_inputs: Vec<crate::text_input::TextInput>,
    pub name_input: crate::text_input::TextInput,
    // Collection picker
    pub show_collection_picker: bool,
    pub picker_cursor: usize,
    // Variables overlay
    pub show_variables: bool,
    pub var_tier: VarTier,
    pub var_cursor: usize,
    pub var_editing: Option<VarEditTarget>,
    pub var_key_input: crate::text_input::TextInput,
    pub var_value_input: crate::text_input::TextInput,
    // Project picker
    pub show_project_picker: bool,
    pub project_picker_cursor: usize,
    pub all_projects: Vec<(curl_tui_core::types::Project, String)>,
    pub show_first_launch: bool,
    // Environment Manager state
    pub show_env_manager: bool,
    pub env_manager_cursor: usize,
    pub env_manager_renaming: Option<usize>,
    pub env_manager_confirm_delete: Option<usize>,
    pub env_manager_name_input: crate::text_input::TextInput,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            open_projects: Vec::new(),
            active_project_idx: None,
            project_tab_scroll: 0,
            active_pane: Pane::Request,
            pane_visible: [true, true, true],
            should_quit: false,
            show_help: false,
            secrets_revealed: false,
            status_message: None,
            input_mode: InputMode::Normal,
            edit_field: None,
            url_input: crate::text_input::TextInput::new(""),
            body_input: crate::text_input::TextInput::new(""),
            header_key_inputs: Vec::new(),
            header_value_inputs: Vec::new(),
            param_key_inputs: Vec::new(),
            param_value_inputs: Vec::new(),
            name_input: crate::text_input::TextInput::new(""),
            show_collection_picker: false,
            picker_cursor: 0,
            show_variables: false,
            var_tier: VarTier::Global,
            var_cursor: 0,
            var_editing: None,
            var_key_input: crate::text_input::TextInput::new(""),
            var_value_input: crate::text_input::TextInput::new(""),
            show_project_picker: false,
            project_picker_cursor: 0,
            all_projects: Vec::new(),
            show_first_launch: false,
            show_env_manager: false,
            env_manager_cursor: 0,
            env_manager_renaming: None,
            env_manager_confirm_delete: None,
            env_manager_name_input: crate::text_input::TextInput::new(""),
        }
    }

    // ── Workspace accessors ──────────────────────────────────────

    pub fn active_workspace(&self) -> Option<&ProjectWorkspace> {
        self.active_project_idx
            .and_then(|i| self.open_projects.get(i))
    }

    pub fn active_workspace_mut(&mut self) -> Option<&mut ProjectWorkspace> {
        self.active_project_idx
            .and_then(|i| self.open_projects.get_mut(i))
    }

    // ── Convenience accessors for backward compatibility ─────────
    // These delegate to the active workspace so that UI code can
    // still call app.collections(), etc.

    pub fn collections(&self) -> &[Collection] {
        self.active_workspace()
            .map(|ws| ws.data.collections.as_slice())
            .unwrap_or(&[])
    }

    pub fn environments(&self) -> &[Environment] {
        self.active_workspace()
            .map(|ws| ws.data.environments.as_slice())
            .unwrap_or(&[])
    }

    pub fn active_environment(&self) -> Option<usize> {
        self.active_workspace()
            .and_then(|ws| ws.data.active_environment)
    }

    pub fn selected_collection(&self) -> Option<usize> {
        self.active_workspace()
            .and_then(|ws| ws.data.selected_collection)
    }

    pub fn selected_request(&self) -> Option<usize> {
        self.active_workspace()
            .and_then(|ws| ws.data.selected_request)
    }

    pub fn current_request(&self) -> Option<&Request> {
        self.active_workspace()
            .and_then(|ws| ws.data.current_request.as_ref())
    }

    pub fn last_response(&self) -> Option<&CurlResponse> {
        self.active_workspace()
            .and_then(|ws| ws.data.last_response.as_ref())
    }

    pub fn request_tab(&self) -> RequestTab {
        self.active_workspace()
            .map(|ws| ws.request_tab)
            .unwrap_or(RequestTab::Headers)
    }

    pub fn response_tab(&self) -> ResponseTab {
        self.active_workspace()
            .map(|ws| ws.response_tab)
            .unwrap_or(ResponseTab::Body)
    }

    pub fn collection_scroll(&self) -> usize {
        self.active_workspace()
            .map(|ws| ws.collection_scroll)
            .unwrap_or(0)
    }

    pub fn response_scroll(&self) -> usize {
        self.active_workspace()
            .map(|ws| ws.response_scroll)
            .unwrap_or(0)
    }

    pub fn var_collection_idx(&self) -> Option<usize> {
        self.active_workspace()
            .and_then(|ws| ws.data.var_collection_idx)
    }

    pub fn var_environment_idx(&self) -> Option<usize> {
        self.active_workspace()
            .and_then(|ws| ws.data.var_environment_idx)
    }

    // ── Pane management ──────────────────────────────────────────

    /// Toggle a pane's visibility. Ensures at least one pane stays visible.
    pub fn toggle_pane(&mut self, index: usize) {
        let visible_count = self.pane_visible.iter().filter(|&&v| v).count();
        if self.pane_visible[index] && visible_count <= 1 {
            return; // Can't hide the last visible pane
        }
        self.pane_visible[index] = !self.pane_visible[index];

        // If active pane was hidden, switch to first visible
        let active_index = match self.active_pane {
            Pane::Collections => 0,
            Pane::Request => 1,
            Pane::Response => 2,
        };
        if !self.pane_visible[active_index] {
            self.cycle_pane_forward();
        }
    }

    pub fn cycle_pane_forward(&mut self) {
        let panes = [Pane::Collections, Pane::Request, Pane::Response];
        let current = panes.iter().position(|p| *p == self.active_pane).unwrap();
        for i in 1..=3 {
            let next = (current + i) % 3;
            if self.pane_visible[next] {
                self.active_pane = panes[next];
                return;
            }
        }
    }

    pub fn cycle_pane_backward(&mut self) {
        let panes = [Pane::Collections, Pane::Request, Pane::Response];
        let current = panes.iter().position(|p| *p == self.active_pane).unwrap();
        for i in 1..=3 {
            let prev = (current + 3 - i) % 3;
            if self.pane_visible[prev] {
                self.active_pane = panes[prev];
                return;
            }
        }
    }

    // ── Request execution ────────────────────────────────────────

    pub async fn send_request(&mut self) {
        let Some(ws) = self.active_workspace() else {
            self.status_message = Some("No project open".to_string());
            return;
        };

        let Some(request) = &ws.data.current_request else {
            self.status_message = Some("No request to send".to_string());
            return;
        };

        if request.url.is_empty() {
            self.status_message = Some("URL is empty".to_string());
            return;
        }

        // Extract data before the async call to avoid borrow issues
        let global_vars = self.config.variables.clone();
        let env_vars = ws
            .data
            .active_environment
            .and_then(|i| ws.data.environments.get(i))
            .map(|e| e.variables.clone());
        let col_vars = ws
            .data
            .selected_collection
            .and_then(|i| ws.data.collections.get(i))
            .map(|c| c.variables.clone());
        let request = request.clone();
        let collection_id = ws
            .data
            .selected_collection
            .and_then(|i| ws.data.collections.get(i))
            .map(|c| c.id);
        let env_name = ws
            .data
            .active_environment
            .and_then(|i| ws.data.environments.get(i))
            .map(|e| e.name.clone());
        let project_id = Some(ws.data.project.id);
        let project_name = Some(ws.data.project.name.clone());
        let slug = ws.data.slug.clone();
        let default_timeout = self.config.default_timeout;
        let max_size = self.config.max_response_body_size_bytes as usize;

        let resolver = FileVariableResolver::new(global_vars, env_vars, col_vars);

        // Resolve URL
        let (resolved_url, mut secrets) = match resolver.resolve(&request.url) {
            Ok(r) => r,
            Err(e) => {
                self.status_message = Some(format!("Variable error: {}", e));
                return;
            }
        };

        // Build command
        let mut builder = CurlCommandBuilder::new(&resolved_url)
            .method(request.method)
            .timeout(default_timeout);

        // Add headers
        for header in &request.headers {
            if header.enabled {
                let (resolved_val, s) = match resolver.resolve(&header.value) {
                    Ok(r) => r,
                    Err(e) => {
                        self.status_message = Some(format!("Header variable error: {}", e));
                        return;
                    }
                };
                secrets.extend(s);
                builder = builder.header(&header.key, &resolved_val);
            }
        }

        // Add query params
        for param in &request.params {
            if param.enabled {
                let (resolved_val, s) = match resolver.resolve(&param.value) {
                    Ok(r) => r,
                    Err(e) => {
                        self.status_message = Some(format!("Param variable error: {}", e));
                        return;
                    }
                };
                secrets.extend(s);
                builder = builder.query_param(&param.key, &resolved_val);
            }
        }

        // Add body
        if let Some(body) = &request.body {
            match body {
                Body::Json { content } => {
                    let (resolved, s) = match resolver.resolve(content) {
                        Ok(r) => r,
                        Err(e) => {
                            self.status_message = Some(format!("Body variable error: {}", e));
                            return;
                        }
                    };
                    secrets.extend(s);
                    builder = builder.body_json(&resolved);
                }
                Body::Text { content } => {
                    builder = builder.body_text(content);
                }
                Body::Form { fields } => {
                    for field in fields {
                        if field.enabled {
                            let (val, s) = match resolver.resolve(&field.value) {
                                Ok(r) => r,
                                Err(e) => {
                                    self.status_message =
                                        Some(format!("Form variable error: {}", e));
                                    return;
                                }
                            };
                            secrets.extend(s);
                            builder = builder.form_field(&field.key, &val);
                        }
                    }
                }
                Body::Multipart { parts } => {
                    for part in parts {
                        if let Some(file_path) = &part.file_path {
                            builder = builder.multipart_file(&part.name, file_path);
                        } else if let Some(value) = &part.value {
                            builder = builder.multipart_field(&part.name, value);
                        }
                    }
                }
                Body::None => {}
            }
        }

        // Add auth
        if let Some(auth) = &request.auth {
            match auth {
                Auth::Bearer { token } => {
                    let (val, s) = match resolver.resolve(token) {
                        Ok(r) => r,
                        Err(e) => {
                            self.status_message = Some(format!("Auth variable error: {}", e));
                            return;
                        }
                    };
                    secrets.extend(s);
                    builder = builder.header("Authorization", &format!("Bearer {}", val));
                }
                Auth::Basic { username, password } => {
                    let (user, _) = match resolver.resolve(username) {
                        Ok(r) => r,
                        Err(e) => {
                            self.status_message = Some(format!("Auth variable error: {}", e));
                            return;
                        }
                    };
                    let (pass, s) = match resolver.resolve(password) {
                        Ok(r) => r,
                        Err(e) => {
                            self.status_message = Some(format!("Auth variable error: {}", e));
                            return;
                        }
                    };
                    secrets.extend(s);
                    builder = builder.basic_auth(&user, &pass);
                }
                Auth::ApiKey {
                    key,
                    value,
                    location,
                } => {
                    let (val, s) = match resolver.resolve(value) {
                        Ok(r) => r,
                        Err(e) => {
                            self.status_message = Some(format!("Auth variable error: {}", e));
                            return;
                        }
                    };
                    secrets.extend(s);
                    match location {
                        curl_tui_core::types::ApiKeyLocation::Header => {
                            builder = builder.header(key, &val);
                        }
                        curl_tui_core::types::ApiKeyLocation::Query => {
                            builder = builder.query_param(key, &val);
                        }
                    }
                }
                Auth::None => {}
            }
        }

        self.status_message = Some("Sending...".to_string());
        let cmd = builder.build();

        match cmd.execute().await {
            Ok(response) => {
                let mut resp = response;
                if resp.body.len() > max_size {
                    resp.body.truncate(max_size);
                    resp.body
                        .push_str("\n\n[TRUNCATED — response exceeded size limit]");
                }
                // Update display command with redaction
                resp.raw_command = cmd.to_display_string(&secrets);

                self.status_message = Some(format!(
                    "{} {} — {:.0}ms",
                    resp.status_code, request.method, resp.timing.total_ms
                ));

                // Log to history (dual: global + project)
                let global_history = config_dir().join("history.jsonl");
                let project_history = config_dir()
                    .join("projects")
                    .join(&slug)
                    .join("history.jsonl");
                let entry = HistoryEntry {
                    id: uuid::Uuid::new_v4(),
                    timestamp: chrono::Utc::now(),
                    collection_id,
                    request_name: request.name.clone(),
                    method: request.method,
                    url: resolved_url,
                    status_code: Some(resp.status_code),
                    duration_ms: Some(resp.timing.total_ms as u64),
                    environment: env_name,
                    project_id,
                    project_name,
                };
                let _ = append_entry_dual(&global_history, &project_history, &entry, &secrets);

                // Write response back to workspace
                if let Some(ws) = self.active_workspace_mut() {
                    ws.data.last_response = Some(resp);
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Error: {}", e));
            }
        }
    }

    // ── Save / collection management ─────────────────────────────

    pub fn save_current_request(&mut self) {
        {
            let Some(ws) = self.active_workspace() else {
                self.status_message = Some("No project open".to_string());
                return;
            };
            if ws.data.current_request.is_none() {
                self.status_message = Some("No request to save".to_string());
                return;
            }
        }

        let collections_len = self
            .active_workspace()
            .map(|ws| ws.data.collections.len())
            .unwrap_or(0);
        let selected_collection = self
            .active_workspace()
            .and_then(|ws| ws.data.selected_collection);

        if collections_len == 0 {
            // No collections at all — prompt to create one
            self.name_input.set_content("My Collection");
            self.start_editing(EditField::NewCollectionName);
            self.status_message =
                Some("Name your collection, then press Enter to save".to_string());
        } else if collections_len == 1 && selected_collection == Some(0) {
            // Only one collection and it's selected — save directly
            self.save_request_to_collection(0);
        } else {
            // Multiple collections or none selected — show picker
            self.picker_cursor = selected_collection.unwrap_or(0);
            self.show_collection_picker = true;
            self.status_message = Some("Choose a collection to save into".to_string());
        }
    }

    /// Save the current request into a specific collection by index
    pub fn save_request_to_collection(&mut self, col_idx: usize) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        let Some(request) = &ws.data.current_request else {
            return;
        };

        if let Some(collection) = ws.data.collections.get_mut(col_idx) {
            // If this request already exists in this collection (same id), update it
            let existing_idx = collection.requests.iter().position(|r| r.id == request.id);

            if let Some(req_idx) = existing_idx {
                collection.requests[req_idx] = request.clone();
                ws.data.selected_request = Some(req_idx);
            } else {
                collection.requests.push(request.clone());
                ws.data.selected_request = Some(collection.requests.len() - 1);
            }

            ws.data.selected_collection = Some(col_idx);
            let collections_dir = config_dir()
                .join("projects")
                .join(&ws.data.slug)
                .join("collections");
            match curl_tui_core::collection::save_collection(&collections_dir, collection) {
                Ok(_) => self.status_message = Some(format!("Saved to '{}'!", collection.name)),
                Err(e) => self.status_message = Some(format!("Save error: {}", e)),
            }
        }
    }

    pub fn create_new_collection(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        let collection = Collection {
            id: uuid::Uuid::new_v4(),
            name: "New Collection".to_string(),
            variables: std::collections::HashMap::new(),
            requests: Vec::new(),
        };
        let collections_dir = config_dir()
            .join("projects")
            .join(&ws.data.slug)
            .join("collections");
        match curl_tui_core::collection::save_collection(&collections_dir, &collection) {
            Ok(_) => {
                ws.data.collections.push(collection);
                let idx = ws.data.collections.len() - 1;
                ws.data.selected_collection = Some(idx);
                ws.data.selected_request = None;
                self.name_input.set_content("New Collection");
                self.start_editing(EditField::CollectionName(idx));
                self.status_message = Some("Name your collection, then press Enter".to_string());
            }
            Err(e) => {
                self.status_message = Some(format!("Error creating collection: {}", e));
            }
        }
    }

    pub fn delete_selected_in_collections(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        let Some(col_idx) = ws.data.selected_collection else {
            self.status_message = Some("Nothing selected".to_string());
            return;
        };

        if let Some(req_idx) = ws.data.selected_request {
            // Delete a request from the collection
            if let Some(collection) = ws.data.collections.get_mut(col_idx) {
                if req_idx < collection.requests.len() {
                    let name = collection.requests[req_idx].name.clone();
                    collection.requests.remove(req_idx);

                    // Save collection to disk
                    let collections_dir = config_dir()
                        .join("projects")
                        .join(&ws.data.slug)
                        .join("collections");
                    let _ =
                        curl_tui_core::collection::save_collection(&collections_dir, collection);

                    // Adjust selection
                    if collection.requests.is_empty() {
                        ws.data.selected_request = None;
                    } else if req_idx >= collection.requests.len() {
                        ws.data.selected_request = Some(collection.requests.len() - 1);
                    }

                    // Clear current request if it was the deleted one
                    if ws
                        .data
                        .current_request
                        .as_ref()
                        .is_some_and(|r| r.name == name)
                    {
                        ws.data.current_request = None;
                        ws.data.last_response = None;
                    }

                    self.status_message = Some(format!("Deleted request '{}'", name));
                }
            }
        } else {
            // Delete the entire collection
            if let Some(collection) = ws.data.collections.get(col_idx) {
                let name = collection.name.clone();
                let slug_str = curl_tui_core::collection::slugify(&name);
                let path = config_dir()
                    .join("projects")
                    .join(&ws.data.slug)
                    .join("collections")
                    .join(format!("{}.json", slug_str));
                if path.exists() {
                    let _ = std::fs::remove_file(&path);
                }

                ws.data.collections.remove(col_idx);

                // Adjust all collection indices
                if ws.data.collections.is_empty() {
                    ws.data.selected_collection = None;
                    ws.data.var_collection_idx = None;
                } else {
                    let max = ws.data.collections.len() - 1;
                    if col_idx > max {
                        ws.data.selected_collection = Some(max);
                    }
                    ws.data.var_collection_idx = ws.data.var_collection_idx.map(|i| i.min(max));
                }
                ws.data.selected_request = None;

                self.status_message = Some(format!("Deleted collection '{}'", name));
            }
        }
    }

    pub fn new_request(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        ws.data.current_request = Some(Request {
            id: uuid::Uuid::new_v4(),
            name: "New Request".to_string(),
            method: Method::Get,
            url: String::new(),
            headers: Vec::new(),
            params: Vec::new(),
            body: None,
            auth: None,
        });
        ws.data.selected_request = None;
        ws.data.last_response = None;
        // Start editing the request name immediately
        self.name_input.set_content("New Request");
        self.start_editing(EditField::RequestName);
        self.active_pane = Pane::Request;
        self.status_message = Some("Name your request, then press Enter".to_string());
    }

    /// Enter editing mode for a specific field
    pub fn start_editing(&mut self, field: EditField) {
        self.input_mode = InputMode::Editing;
        self.edit_field = Some(field);
    }

    /// Exit editing mode, syncing text input back to request
    pub fn stop_editing(&mut self) {
        if let Some(field) = self.edit_field.take() {
            if field == EditField::NewCollectionName {
                self.finalize_new_collection();
            } else if field == EditField::NewProjectName {
                self.finalize_new_project();
            } else {
                self.sync_field_to_request(field);
            }
        }
        self.input_mode = InputMode::Normal;
    }

    /// Create a new collection with the name from name_input and save current request into it
    fn finalize_new_collection(&mut self) {
        let name = self.name_input.content().to_string();
        let name = if name.is_empty() {
            "My Collection".to_string()
        } else {
            name
        };

        let Some(ws) = self.active_workspace_mut() else {
            return;
        };

        let request = match &ws.data.current_request {
            Some(r) => r.clone(),
            None => return,
        };

        let new_collection = Collection {
            id: uuid::Uuid::new_v4(),
            name: name.clone(),
            variables: std::collections::HashMap::new(),
            requests: vec![request],
        };
        let collections_dir = config_dir()
            .join("projects")
            .join(&ws.data.slug)
            .join("collections");
        match curl_tui_core::collection::save_collection(&collections_dir, &new_collection) {
            Ok(_) => {
                ws.data.collections.push(new_collection);
                let col_idx = ws.data.collections.len() - 1;
                ws.data.selected_collection = Some(col_idx);
                ws.data.selected_request = Some(0);
                self.status_message = Some(format!("Created '{}' and saved!", name));
            }
            Err(e) => self.status_message = Some(format!("Save error: {}", e)),
        }
    }

    fn finalize_new_project(&mut self) {
        let name = self.name_input.content().to_string();
        let name = if name.is_empty() {
            "New Project".to_string()
        } else {
            name
        };
        let project = curl_tui_core::types::Project {
            id: uuid::Uuid::new_v4(),
            name: name.clone(),
            active_environment: None,
        };
        let projects_dir = config_dir().join("projects");
        match curl_tui_core::project::create_project(&projects_dir, &project) {
            Ok(dir) => {
                let slug = dir.file_name().unwrap().to_string_lossy().to_string();
                let ws = ProjectWorkspace::new(project, slug);
                self.open_projects.push(ws);
                let idx = self.open_projects.len() - 1;
                self.switch_project(idx);
                self.status_message = Some(format!("Created project '{}'", name));
            }
            Err(e) => self.status_message = Some(format!("Error: {}", e)),
        }
    }

    /// Get a mutable reference to the active TextInput
    pub fn active_text_input(&mut self) -> Option<&mut crate::text_input::TextInput> {
        match self.edit_field? {
            EditField::Url => Some(&mut self.url_input),
            EditField::BodyContent => Some(&mut self.body_input),
            EditField::HeaderKey(i) => self.header_key_inputs.get_mut(i),
            EditField::HeaderValue(i) => self.header_value_inputs.get_mut(i),
            EditField::ParamKey(i) => self.param_key_inputs.get_mut(i),
            EditField::ParamValue(i) => self.param_value_inputs.get_mut(i),
            EditField::RequestName
            | EditField::CollectionName(_)
            | EditField::EnvironmentName(_)
            | EditField::NewCollectionName
            | EditField::NewProjectName => Some(&mut self.name_input),
        }
    }

    /// Sync the text input content back to the current request
    fn sync_field_to_request(&mut self, field: EditField) {
        // For fields that need workspace access, handle them with workspace
        match field {
            EditField::Url => {
                let url = self.url_input.content().to_string();
                if let Some(ws) = self.active_workspace_mut() {
                    if let Some(request) = &mut ws.data.current_request {
                        request.url = url;
                    }
                }
            }
            EditField::BodyContent => {
                let content = self.body_input.content().to_string();
                if let Some(ws) = self.active_workspace_mut() {
                    if let Some(request) = &mut ws.data.current_request {
                        if content.is_empty() {
                            request.body = Option::None;
                        } else {
                            request.body = Some(curl_tui_core::types::Body::Json { content });
                        }
                    }
                }
            }
            EditField::HeaderKey(i) => {
                let val = self
                    .header_key_inputs
                    .get(i)
                    .map(|inp| inp.content().to_string());
                if let Some(val) = val {
                    if let Some(ws) = self.active_workspace_mut() {
                        if let Some(request) = &mut ws.data.current_request {
                            if let Some(header) = request.headers.get_mut(i) {
                                header.key = val;
                            }
                        }
                    }
                }
            }
            EditField::HeaderValue(i) => {
                let val = self
                    .header_value_inputs
                    .get(i)
                    .map(|inp| inp.content().to_string());
                if let Some(val) = val {
                    if let Some(ws) = self.active_workspace_mut() {
                        if let Some(request) = &mut ws.data.current_request {
                            if let Some(header) = request.headers.get_mut(i) {
                                header.value = val;
                            }
                        }
                    }
                }
            }
            EditField::ParamKey(i) => {
                let val = self
                    .param_key_inputs
                    .get(i)
                    .map(|inp| inp.content().to_string());
                if let Some(val) = val {
                    if let Some(ws) = self.active_workspace_mut() {
                        if let Some(request) = &mut ws.data.current_request {
                            if let Some(param) = request.params.get_mut(i) {
                                param.key = val;
                            }
                        }
                    }
                }
            }
            EditField::ParamValue(i) => {
                let val = self
                    .param_value_inputs
                    .get(i)
                    .map(|inp| inp.content().to_string());
                if let Some(val) = val {
                    if let Some(ws) = self.active_workspace_mut() {
                        if let Some(request) = &mut ws.data.current_request {
                            if let Some(param) = request.params.get_mut(i) {
                                param.value = val;
                            }
                        }
                    }
                }
            }
            EditField::RequestName => {
                let name = self.name_input.content().to_string();
                if !name.is_empty() {
                    if let Some(ws) = self.active_workspace_mut() {
                        if let Some(request) = &mut ws.data.current_request {
                            request.name = name.clone();
                        }
                        // Auto-save the renamed request to its collection
                        if let Some(col_idx) = ws.data.selected_collection {
                            if let Some(req_idx) = ws.data.selected_request {
                                if let Some(collection) = ws.data.collections.get_mut(col_idx) {
                                    if let Some(existing) = collection.requests.get_mut(req_idx) {
                                        existing.name = name;
                                    }
                                    let collections_dir = config_dir()
                                        .join("projects")
                                        .join(&ws.data.slug)
                                        .join("collections");
                                    match curl_tui_core::collection::save_collection(
                                        &collections_dir,
                                        collection,
                                    ) {
                                        Ok(_) => {
                                            self.status_message =
                                                Some("Renamed and saved!".to_string())
                                        }
                                        Err(e) => {
                                            self.status_message =
                                                Some(format!("Rename ok, save error: {}", e))
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            EditField::CollectionName(col_idx) => {
                let name = self.name_input.content().to_string();
                if !name.is_empty() {
                    if let Some(ws) = self.active_workspace_mut() {
                        if let Some(collection) = ws.data.collections.get_mut(col_idx) {
                            // Delete old file if name changed (slug differs)
                            let old_slug = curl_tui_core::collection::slugify(&collection.name);
                            let new_slug = curl_tui_core::collection::slugify(&name);
                            let collections_dir = config_dir()
                                .join("projects")
                                .join(&ws.data.slug)
                                .join("collections");
                            if old_slug != new_slug {
                                let old_path = collections_dir.join(format!("{}.json", old_slug));
                                if old_path.exists() {
                                    let _ = std::fs::remove_file(&old_path);
                                }
                            }

                            collection.name = name;
                            let _ = curl_tui_core::collection::save_collection(
                                &collections_dir,
                                collection,
                            );
                        }
                    }
                }
            }
            EditField::EnvironmentName(env_idx) => {
                let name = self.name_input.content().to_string();
                if !name.is_empty() {
                    if let Some(ws) = self.active_workspace_mut() {
                        if let Some(env) = ws.data.environments.get_mut(env_idx) {
                            // Delete old file if name changed (slug differs)
                            let old_slug = curl_tui_core::collection::slugify(&env.name);
                            let new_slug = curl_tui_core::collection::slugify(&name);
                            let env_dir = config_dir()
                                .join("projects")
                                .join(&ws.data.slug)
                                .join("environments");
                            if old_slug != new_slug {
                                let old_path = env_dir.join(format!("{}.json", old_slug));
                                if old_path.exists() {
                                    let _ = std::fs::remove_file(&old_path);
                                }
                            }

                            env.name = name.clone();
                            match curl_tui_core::environment::save_environment(&env_dir, env) {
                                Ok(_) => {
                                    self.status_message =
                                        Some(format!("Environment '{}' saved!", name))
                                }
                                Err(e) => self.status_message = Some(format!("Save error: {}", e)),
                            }
                        }
                    }
                }
            }
            EditField::NewCollectionName | EditField::NewProjectName => {
                // Handled separately — sync is a no-op here
            }
        }
    }

    /// Load a request's fields into the text inputs
    pub fn load_request_into_inputs(&mut self) {
        let request = self
            .active_workspace()
            .and_then(|ws| ws.data.current_request.clone());
        if let Some(request) = &request {
            self.url_input.set_content(&request.url);
            match &request.body {
                Some(curl_tui_core::types::Body::Json { content }) => {
                    self.body_input.set_content(content);
                }
                Some(curl_tui_core::types::Body::Text { content }) => {
                    self.body_input.set_content(content);
                }
                _ => {
                    self.body_input.clear();
                }
            }
            // Populate header inputs
            self.header_key_inputs = request
                .headers
                .iter()
                .map(|h| crate::text_input::TextInput::new(&h.key))
                .collect();
            self.header_value_inputs = request
                .headers
                .iter()
                .map(|h| crate::text_input::TextInput::new(&h.value))
                .collect();
            // Populate param inputs
            self.param_key_inputs = request
                .params
                .iter()
                .map(|p| crate::text_input::TextInput::new(&p.key))
                .collect();
            self.param_value_inputs = request
                .params
                .iter()
                .map(|p| crate::text_input::TextInput::new(&p.value))
                .collect();
        }
    }

    /// Rename the currently selected item in the Collections pane,
    /// or the current request name if in the Request pane
    pub fn handle_rename(&mut self) {
        match self.active_pane {
            Pane::Collections => {
                // Clone out the data we need to avoid borrow issues
                let rename_info = self.active_workspace().and_then(|ws| {
                    let col_idx = ws.data.selected_collection?;
                    if let Some(req_idx) = ws.data.selected_request {
                        let req = ws
                            .data
                            .collections
                            .get(col_idx)?
                            .requests
                            .get(req_idx)?
                            .clone();
                        Some((col_idx, Some((req_idx, req))))
                    } else {
                        let _col_name = ws.data.collections.get(col_idx)?.name.clone();
                        Some((col_idx, Option::<(usize, Request)>::None))
                    }
                });

                if let Some((col_idx, req_info)) = rename_info {
                    if let Some((_req_idx, req)) = req_info {
                        // Rename a request
                        self.name_input.set_content(&req.name);
                        if let Some(ws) = self.active_workspace_mut() {
                            ws.data.current_request = Some(req);
                        }
                        self.load_request_into_inputs();
                        self.start_editing(EditField::RequestName);
                        self.status_message = Some("Rename request".to_string());
                    } else {
                        // Rename a collection
                        let col_name = self
                            .active_workspace()
                            .and_then(|ws| ws.data.collections.get(col_idx))
                            .map(|c| c.name.clone());
                        if let Some(name) = col_name {
                            self.name_input.set_content(&name);
                            self.start_editing(EditField::CollectionName(col_idx));
                            self.status_message = Some("Rename collection".to_string());
                        }
                    }
                }
            }
            Pane::Request => {
                // Rename current request
                let req_name = self
                    .active_workspace()
                    .and_then(|ws| ws.data.current_request.as_ref())
                    .map(|r| r.name.clone());
                if let Some(name) = req_name {
                    self.name_input.set_content(&name);
                    self.start_editing(EditField::RequestName);
                    self.status_message = Some("Rename request".to_string());
                }
            }
            _ => {}
        }
    }

    /// Add a new empty header to the current request
    pub fn add_header(&mut self) {
        let header_idx = {
            let Some(ws) = self.active_workspace_mut() else {
                return;
            };
            let Some(request) = &mut ws.data.current_request else {
                return;
            };
            request.headers.push(curl_tui_core::types::Header {
                key: String::new(),
                value: String::new(),
                enabled: true,
            });
            request.headers.len() - 1
        };
        self.header_key_inputs
            .push(crate::text_input::TextInput::default());
        self.header_value_inputs
            .push(crate::text_input::TextInput::default());
        self.start_editing(EditField::HeaderKey(header_idx));
    }

    /// Add a new empty param to the current request
    pub fn add_param(&mut self) {
        let param_idx = {
            let Some(ws) = self.active_workspace_mut() else {
                return;
            };
            let Some(request) = &mut ws.data.current_request else {
                return;
            };
            request.params.push(curl_tui_core::types::Param {
                key: String::new(),
                value: String::new(),
                enabled: true,
            });
            request.params.len() - 1
        };
        self.param_key_inputs
            .push(crate::text_input::TextInput::default());
        self.param_value_inputs
            .push(crate::text_input::TextInput::default());
        self.start_editing(EditField::ParamKey(param_idx));
    }

    /// Handle Enter in Normal mode based on active pane
    pub fn handle_enter(&mut self) {
        match self.active_pane {
            Pane::Collections => {
                // Load the selected request
                let req_clone = self.active_workspace().and_then(|ws| {
                    let col_idx = ws.data.selected_collection?;
                    let req_idx = ws.data.selected_request?;
                    ws.data
                        .collections
                        .get(col_idx)?
                        .requests
                        .get(req_idx)
                        .cloned()
                });
                if let Some(req) = req_clone {
                    let name = req.name.clone();
                    if let Some(ws) = self.active_workspace_mut() {
                        ws.data.current_request = Some(req);
                    }
                    self.load_request_into_inputs();
                    self.active_pane = Pane::Request;
                    self.status_message = Some(format!("Loaded: {}", name));
                }
            }
            Pane::Request => {
                // Start editing the URL field
                self.start_editing(EditField::Url);
            }
            _ => {}
        }
    }

    /// Handle MoveUp in Normal mode based on active pane
    pub fn handle_move_up(&mut self) {
        match self.active_pane {
            Pane::Collections => {
                self.move_collection_cursor_up();
                self.adjust_collection_scroll(20); // approximate; UI will clamp
            }
            Pane::Request => {
                // Could move between header rows in future
            }
            Pane::Response => {
                if let Some(ws) = self.active_workspace_mut() {
                    if ws.response_scroll > 0 {
                        ws.response_scroll -= 1;
                    }
                }
            }
        }
    }

    /// Handle MoveDown in Normal mode based on active pane
    pub fn handle_move_down(&mut self) {
        match self.active_pane {
            Pane::Collections => {
                self.move_collection_cursor_down();
                self.adjust_collection_scroll(20); // approximate; UI will clamp
            }
            Pane::Request => {
                // Could move between header rows in future
            }
            Pane::Response => {
                if let Some(ws) = self.active_workspace_mut() {
                    ws.response_scroll = ws.response_scroll.saturating_add(1);
                }
            }
        }
    }

    /// Calculate the flat index of the current collection cursor position
    fn collection_cursor_flat_index(&self) -> usize {
        let Some(ws) = self.active_workspace() else {
            return 0;
        };
        let mut idx = 0;
        for (col_idx, col) in ws.data.collections.iter().enumerate() {
            if Some(col_idx) == ws.data.selected_collection && ws.data.selected_request.is_none() {
                return idx;
            }
            idx += 1; // collection row
            for (req_idx, _) in col.requests.iter().enumerate() {
                if Some(col_idx) == ws.data.selected_collection
                    && Some(req_idx) == ws.data.selected_request
                {
                    return idx;
                }
                idx += 1;
            }
        }
        idx
    }

    /// Adjust collection_scroll to keep the cursor visible
    fn adjust_collection_scroll(&mut self, visible_height: usize) {
        let cursor = self.collection_cursor_flat_index();
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        if cursor < ws.collection_scroll {
            ws.collection_scroll = cursor;
        } else if cursor >= ws.collection_scroll + visible_height {
            ws.collection_scroll = cursor - visible_height + 1;
        }
    }

    /// Move collection cursor up through the flat list of collections and their requests
    fn move_collection_cursor_up(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        if let Some(req_idx) = ws.data.selected_request {
            if req_idx > 0 {
                ws.data.selected_request = Some(req_idx - 1);
            } else {
                // Move back to collection level
                ws.data.selected_request = Option::None;
            }
        } else if let Some(col_idx) = ws.data.selected_collection {
            if col_idx > 0 {
                ws.data.selected_collection = Some(col_idx - 1);
                // Select last request of previous collection
                if let Some(col) = ws.data.collections.get(col_idx - 1) {
                    if !col.requests.is_empty() {
                        ws.data.selected_request = Some(col.requests.len() - 1);
                    }
                }
            }
        } else if !ws.data.collections.is_empty() {
            ws.data.selected_collection = Some(0);
        }
    }

    /// Move collection cursor down through the flat list
    fn move_collection_cursor_down(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        if let Some(col_idx) = ws.data.selected_collection {
            if let Some(collection) = ws.data.collections.get(col_idx) {
                let requests_len = collection.requests.len();
                if let Some(req_idx) = ws.data.selected_request {
                    if req_idx + 1 < requests_len {
                        ws.data.selected_request = Some(req_idx + 1);
                    } else if col_idx + 1 < ws.data.collections.len() {
                        // Move to next collection
                        ws.data.selected_collection = Some(col_idx + 1);
                        ws.data.selected_request = Option::None;
                    }
                } else if !collection.requests.is_empty() {
                    ws.data.selected_request = Some(0);
                } else if col_idx + 1 < ws.data.collections.len() {
                    ws.data.selected_collection = Some(col_idx + 1);
                }
            }
        } else if !ws.data.collections.is_empty() {
            ws.data.selected_collection = Some(0);
        }
    }

    /// Switch to next request tab
    pub fn next_request_tab(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        ws.request_tab = match ws.request_tab {
            RequestTab::Headers => RequestTab::Body,
            RequestTab::Body => RequestTab::Auth,
            RequestTab::Auth => RequestTab::Params,
            RequestTab::Params => RequestTab::Headers,
        };
    }

    /// Switch to previous request tab
    pub fn prev_request_tab(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        ws.request_tab = match ws.request_tab {
            RequestTab::Headers => RequestTab::Params,
            RequestTab::Body => RequestTab::Headers,
            RequestTab::Auth => RequestTab::Body,
            RequestTab::Params => RequestTab::Auth,
        };
    }

    /// Switch to next response tab
    pub fn next_response_tab(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        ws.response_tab = match ws.response_tab {
            ResponseTab::Body => ResponseTab::Headers,
            ResponseTab::Headers => ResponseTab::Timing,
            ResponseTab::Timing => ResponseTab::Body,
        };
    }

    /// Switch to previous response tab
    pub fn prev_response_tab(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        ws.response_tab = match ws.response_tab {
            ResponseTab::Body => ResponseTab::Timing,
            ResponseTab::Headers => ResponseTab::Body,
            ResponseTab::Timing => ResponseTab::Headers,
        };
    }

    /// Sync the active environment name to project.json after any environment index change.
    fn persist_active_environment(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        ws.data.sync_active_environment_name();
        let project_dir = config_dir().join("projects").join(&ws.data.slug);
        let _ = curl_tui_core::project::save_project(&project_dir, &ws.data.project);
    }

    /// Open the environment manager modal, reset state.
    pub fn open_env_manager(&mut self) {
        self.show_env_manager = true;
        self.env_manager_cursor = 0;
        self.env_manager_renaming = None;
        self.env_manager_confirm_delete = None;
    }

    /// Create a new environment from the env manager modal (does not auto-activate).
    pub fn env_manager_create(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        let env = Environment {
            id: uuid::Uuid::new_v4(),
            name: "New Environment".to_string(),
            variables: std::collections::HashMap::new(),
        };
        let env_dir = config_dir()
            .join("projects")
            .join(&ws.data.slug)
            .join("environments");
        match curl_tui_core::environment::save_environment(&env_dir, &env) {
            Ok(_) => {
                ws.data.environments.push(env);
                let idx = ws.data.environments.len() - 1;
                self.env_manager_cursor = idx;
                self.env_manager_name_input.set_content("New Environment");
                self.env_manager_renaming = Some(idx);
            }
            Err(e) => {
                self.status_message = Some(format!("Error creating environment: {}", e));
            }
        }
    }

    /// Activate the environment at the cursor and close the modal.
    pub fn env_manager_activate(&mut self) {
        let cursor = self.env_manager_cursor;
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        if let Some(env) = ws.data.environments.get(cursor) {
            let name = env.name.clone();
            ws.data.active_environment = Some(cursor);
            ws.data.var_environment_idx = Some(cursor);
            let _ = ws;
            self.persist_active_environment();
            self.show_env_manager = false;
            self.status_message = Some(format!("Environment: {}", name));
        }
    }

    /// Enter rename mode for the environment at the cursor.
    pub fn env_manager_start_rename(&mut self) {
        let cursor = self.env_manager_cursor;
        let name = self
            .active_workspace()
            .and_then(|ws| ws.data.environments.get(cursor))
            .map(|env| env.name.clone());
        if let Some(name) = name {
            self.env_manager_name_input.set_content(&name);
            self.env_manager_renaming = Some(cursor);
        }
    }

    /// Commit the rename: delete old file, save new, update name in memory.
    pub fn env_manager_confirm_rename(&mut self) {
        let Some(rename_idx) = self.env_manager_renaming else {
            return;
        };
        let new_name = self.env_manager_name_input.content().to_string();
        if new_name.is_empty() {
            self.env_manager_renaming = None;
            return;
        }
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        if let Some(env) = ws.data.environments.get_mut(rename_idx) {
            let old_slug = curl_tui_core::collection::slugify(&env.name);
            let env_dir = config_dir()
                .join("projects")
                .join(&ws.data.slug)
                .join("environments");
            // Delete old file
            let old_path = env_dir.join(format!("{}.json", old_slug));
            if old_path.exists() {
                let _ = std::fs::remove_file(&old_path);
            }
            // Update name and save new file
            env.name = new_name;
            let _ = curl_tui_core::environment::save_environment(&env_dir, env);
        }
        self.env_manager_renaming = None;
        // If the renamed env is the active one, update project.json
        if self
            .active_workspace()
            .and_then(|ws| ws.data.active_environment)
            == Some(rename_idx)
        {
            self.persist_active_environment();
        }
    }

    /// Show delete confirmation for the environment at the cursor.
    pub fn env_manager_request_delete(&mut self) {
        let env_count = self
            .active_workspace()
            .map(|ws| ws.data.environments.len())
            .unwrap_or(0);
        if self.env_manager_cursor < env_count {
            self.env_manager_confirm_delete = Some(self.env_manager_cursor);
        }
    }

    /// Execute the confirmed deletion.
    pub fn env_manager_execute_delete(&mut self) {
        let Some(delete_idx) = self.env_manager_confirm_delete.take() else {
            return;
        };
        let cursor = self.env_manager_cursor;
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        let Some(env) = ws.data.environments.get(delete_idx) else {
            return;
        };

        // Delete the file from disk
        let slug_str = curl_tui_core::collection::slugify(&env.name);
        let path = config_dir()
            .join("projects")
            .join(&ws.data.slug)
            .join("environments")
            .join(format!("{}.json", slug_str));
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }

        let name = env.name.clone();
        ws.data.environments.remove(delete_idx);

        // Adjust active_environment index
        if ws.data.environments.is_empty() {
            ws.data.active_environment = None;
            ws.data.var_environment_idx = None;
        } else {
            let max = ws.data.environments.len() - 1;
            ws.data.active_environment = ws.data.active_environment.map(|i| {
                if i == delete_idx {
                    return max.min(delete_idx); // deleted the active one
                }
                if i > delete_idx {
                    i - 1 // shift down
                } else {
                    i
                }
            });
            ws.data.var_environment_idx = ws.data.active_environment;
        }

        // Compute new cursor value while ws is still in scope
        let new_cursor = if ws.data.environments.is_empty() {
            0
        } else {
            let max = ws.data.environments.len() - 1;
            cursor.min(max)
        };

        let _ = ws;
        self.env_manager_cursor = new_cursor;
        self.persist_active_environment();
        self.status_message = Some(format!("Deleted environment '{}'", name));
    }

    pub fn cycle_environment(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        if ws.data.environments.is_empty() {
            self.status_message =
                Some("No environments. Press Ctrl+Shift+E to manage environments.".to_string());
            return;
        }
        // Cycle: None -> 0 -> 1 -> ... -> N-1 -> None -> 0 -> ...
        ws.data.active_environment = match ws.data.active_environment {
            None => Some(0),
            Some(i) if i + 1 < ws.data.environments.len() => Some(i + 1),
            Some(_) => None, // wrap back to "no environment"
        };
        match &ws.data.active_environment {
            Some(i) => {
                if let Some(env) = ws.data.environments.get(*i) {
                    self.status_message = Some(format!("Environment: {}", env.name));
                }
            }
            None => {
                self.status_message = Some("Environment: None".to_string());
            }
        }
        self.persist_active_environment();
    }

    pub fn create_new_environment(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        let name = if ws.data.environments.is_empty() {
            "Development"
        } else {
            "New Environment"
        };
        let env = Environment {
            id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            variables: std::collections::HashMap::new(),
        };
        let env_dir = config_dir()
            .join("projects")
            .join(&ws.data.slug)
            .join("environments");
        match curl_tui_core::environment::save_environment(&env_dir, &env) {
            Ok(_) => {
                ws.data.environments.push(env);
                let idx = ws.data.environments.len() - 1;
                ws.data.active_environment = Some(idx);
                // Prompt to rename it
                self.name_input.set_content(name);
                self.start_editing(EditField::EnvironmentName(idx));
                self.status_message = Some("Name your environment, then press Enter".to_string());
                self.persist_active_environment();
            }
            Err(e) => {
                self.status_message = Some(format!("Error creating environment: {}", e));
            }
        }
    }

    pub fn delete_active_environment(&mut self) {
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        let idx = match ws.data.var_environment_idx.or(ws.data.active_environment) {
            Some(i) => i,
            None => {
                self.status_message = Some("No environment selected".to_string());
                return;
            }
        };
        let Some(env) = ws.data.environments.get(idx) else {
            return;
        };

        // Delete the file from disk
        let slug_str = curl_tui_core::collection::slugify(&env.name);
        let path = config_dir()
            .join("projects")
            .join(&ws.data.slug)
            .join("environments")
            .join(format!("{}.json", slug_str));
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }

        let name = env.name.clone();
        ws.data.environments.remove(idx);

        // Adjust all environment indices
        if ws.data.environments.is_empty() {
            ws.data.active_environment = None;
            ws.data.var_environment_idx = None;
        } else {
            let max = ws.data.environments.len() - 1;
            ws.data.active_environment = ws.data.active_environment.map(|i| i.min(max));
            ws.data.var_environment_idx = Some(idx.min(max));
        }
        self.var_cursor = 0;

        self.status_message = Some(format!("Deleted environment '{}'", name));
        self.persist_active_environment();
    }

    // ── Project switching ────────────────────────────────────────

    pub fn switch_project(&mut self, idx: usize) {
        if idx >= self.open_projects.len() {
            return;
        }
        self.flush_inputs_to_workspace();
        self.active_project_idx = Some(idx);
        self.load_request_into_inputs();
    }

    pub fn next_project(&mut self) {
        if self.open_projects.is_empty() {
            return;
        }
        let current = self.active_project_idx.unwrap_or(0);
        let next = (current + 1) % self.open_projects.len();
        self.switch_project(next);
    }

    pub fn prev_project(&mut self) {
        if self.open_projects.is_empty() {
            return;
        }
        let current = self.active_project_idx.unwrap_or(0);
        let prev = if current == 0 {
            self.open_projects.len() - 1
        } else {
            current - 1
        };
        self.switch_project(prev);
    }

    pub fn close_project(&mut self, idx: usize) {
        if idx >= self.open_projects.len() {
            return;
        }
        self.open_projects.remove(idx);
        if self.open_projects.is_empty() {
            self.active_project_idx = None;
            self.show_project_picker = true;
        } else if let Some(active) = self.active_project_idx {
            if active >= self.open_projects.len() {
                self.active_project_idx = Some(self.open_projects.len() - 1);
            } else if active > idx {
                self.active_project_idx = Some(active - 1);
            }
            self.load_request_into_inputs();
        }
    }

    fn flush_inputs_to_workspace(&mut self) {
        // Collect all input values first to avoid borrow conflicts
        let url = self.url_input.content().to_string();
        let body_content = self.body_input.content().to_string();
        let header_keys: Vec<String> = self
            .header_key_inputs
            .iter()
            .map(|i| i.content().to_string())
            .collect();
        let header_values: Vec<String> = self
            .header_value_inputs
            .iter()
            .map(|i| i.content().to_string())
            .collect();
        let param_keys: Vec<String> = self
            .param_key_inputs
            .iter()
            .map(|i| i.content().to_string())
            .collect();
        let param_values: Vec<String> = self
            .param_value_inputs
            .iter()
            .map(|i| i.content().to_string())
            .collect();

        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        if let Some(request) = &mut ws.data.current_request {
            request.url = url;
            if body_content.is_empty() {
                request.body = None;
            } else {
                request.body = Some(curl_tui_core::types::Body::Json {
                    content: body_content,
                });
            }
            for (i, header) in request.headers.iter_mut().enumerate() {
                if let Some(k) = header_keys.get(i) {
                    header.key = k.clone();
                }
                if let Some(v) = header_values.get(i) {
                    header.value = v.clone();
                }
            }
            for (i, param) in request.params.iter_mut().enumerate() {
                if let Some(k) = param_keys.get(i) {
                    param.key = k.clone();
                }
                if let Some(v) = param_values.get(i) {
                    param.value = v.clone();
                }
            }
        }
    }

    // ── Variables overlay ──────────────────────────────────────

    /// Get the sorted keys for the current variable tier
    pub fn var_keys(&self) -> Vec<String> {
        let map = match self.var_tier {
            VarTier::Global => &self.config.variables,
            VarTier::Environment => {
                let Some(ws) = self.active_workspace() else {
                    return Vec::new();
                };
                if let Some(env) = ws
                    .data
                    .var_environment_idx
                    .and_then(|i| ws.data.environments.get(i))
                {
                    &env.variables
                } else {
                    return Vec::new();
                }
            }
            VarTier::Collection => {
                let Some(ws) = self.active_workspace() else {
                    return Vec::new();
                };
                if let Some(col) = ws
                    .data
                    .var_collection_idx
                    .and_then(|i| ws.data.collections.get(i))
                {
                    &col.variables
                } else {
                    return Vec::new();
                }
            }
        };
        let mut keys: Vec<String> = map.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// Get a variable by key from the current tier
    pub fn var_get(&self, key: &str) -> Option<&curl_tui_core::types::Variable> {
        match self.var_tier {
            VarTier::Global => self.config.variables.get(key),
            VarTier::Environment => self
                .active_workspace()
                .and_then(|ws| {
                    ws.data
                        .var_environment_idx
                        .and_then(|i| ws.data.environments.get(i))
                })
                .and_then(|e| e.variables.get(key)),
            VarTier::Collection => self
                .active_workspace()
                .and_then(|ws| {
                    ws.data
                        .var_collection_idx
                        .and_then(|i| ws.data.collections.get(i))
                })
                .and_then(|c| c.variables.get(key)),
        }
    }

    pub fn var_move_up(&mut self) {
        if self.var_cursor > 0 {
            self.var_cursor -= 1;
        }
    }

    pub fn var_move_down(&mut self) {
        let count = self.var_keys().len();
        if self.var_cursor + 1 < count {
            self.var_cursor += 1;
        }
    }

    /// Cycle to next collection/environment within the current tier
    pub fn var_cycle_container_forward(&mut self) {
        let tier = self.var_tier;
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        match tier {
            VarTier::Collection => {
                if ws.data.collections.is_empty() {
                    return;
                }
                let next = match ws.data.var_collection_idx {
                    Some(i) => (i + 1) % ws.data.collections.len(),
                    None => 0,
                };
                ws.data.var_collection_idx = Some(next);
            }
            VarTier::Environment => {
                if ws.data.environments.is_empty() {
                    return;
                }
                let next = match ws.data.var_environment_idx {
                    Some(i) => (i + 1) % ws.data.environments.len(),
                    None => 0,
                };
                ws.data.var_environment_idx = Some(next);
            }
            VarTier::Global => return,
        }
        self.var_cursor = 0;
        self.var_editing = None;
    }

    /// Cycle to previous collection/environment within the current tier
    pub fn var_cycle_container_backward(&mut self) {
        let tier = self.var_tier;
        let Some(ws) = self.active_workspace_mut() else {
            return;
        };
        match tier {
            VarTier::Collection => {
                if ws.data.collections.is_empty() {
                    return;
                }
                let prev = match ws.data.var_collection_idx {
                    Some(i) if i > 0 => i - 1,
                    Some(_) => ws.data.collections.len() - 1,
                    None => 0,
                };
                ws.data.var_collection_idx = Some(prev);
                self.var_cursor = 0;
                self.var_editing = None;
            }
            VarTier::Environment => {
                if ws.data.environments.is_empty() {
                    return;
                }
                let prev = match ws.data.var_environment_idx {
                    Some(i) if i > 0 => i - 1,
                    Some(_) => ws.data.environments.len() - 1,
                    None => 0,
                };
                ws.data.var_environment_idx = Some(prev);
                self.var_cursor = 0;
                self.var_editing = None;
            }
            VarTier::Global => {}
        }
    }

    pub fn var_next_tier(&mut self) {
        self.var_tier = match self.var_tier {
            VarTier::Global => VarTier::Environment,
            VarTier::Environment => VarTier::Collection,
            VarTier::Collection => VarTier::Global,
        };
        self.var_cursor = 0;
        self.var_editing = None;
    }

    pub fn var_prev_tier(&mut self) {
        self.var_tier = match self.var_tier {
            VarTier::Global => VarTier::Collection,
            VarTier::Environment => VarTier::Global,
            VarTier::Collection => VarTier::Environment,
        };
        self.var_cursor = 0;
        self.var_editing = None;
    }

    pub fn var_start_edit_key(&mut self) {
        let keys = self.var_keys();
        if let Some(key) = keys.get(self.var_cursor) {
            self.var_key_input.set_content(key);
            self.var_editing = Some(VarEditTarget::Key);
            self.input_mode = InputMode::Editing;
        }
    }

    pub fn var_start_edit_value(&mut self) {
        let keys = self.var_keys();
        if let Some(key) = keys.get(self.var_cursor) {
            let value = self.var_get(key).map(|v| v.value.clone());
            let key = key.clone();
            if let Some(value) = value {
                self.var_value_input.set_content(&value);
                self.var_key_input.set_content(&key);
                self.var_editing = Some(VarEditTarget::Value);
                self.input_mode = InputMode::Editing;
            }
        }
    }

    pub fn var_stop_editing(&mut self) {
        if let Some(target) = self.var_editing.take() {
            let old_key = {
                let keys = self.var_keys();
                keys.get(self.var_cursor).cloned()
            };
            if let Some(old_key) = old_key {
                match target {
                    VarEditTarget::Key => {
                        let new_key = self.var_key_input.content().to_string();
                        if !new_key.is_empty() && new_key != old_key {
                            // Rename: remove old, insert new with same value
                            if let Some(var) = self.var_remove_raw(&old_key) {
                                self.var_insert_raw(&new_key, var);
                            }
                        }
                    }
                    VarEditTarget::Value => {
                        let new_value = self.var_value_input.content().to_string();
                        let key = self.var_key_input.content().to_string();
                        if let Some(var) = self.var_get_mut(&key) {
                            var.value = new_value;
                        }
                    }
                }
                self.var_save_current_tier();
            }
        }
        self.input_mode = InputMode::Normal;
    }

    pub fn var_add(&mut self) {
        let new_key = format!("new_var_{}", self.var_keys().len());
        let var = curl_tui_core::types::Variable {
            value: String::new(),
            secret: false,
        };
        self.var_insert_raw(&new_key, var);
        // Move cursor to the new variable
        let keys = self.var_keys();
        self.var_cursor = keys.iter().position(|k| k == &new_key).unwrap_or(0);
        // Start editing the key
        self.var_key_input.set_content(&new_key);
        self.var_editing = Some(VarEditTarget::Key);
        self.input_mode = InputMode::Editing;
        self.var_save_current_tier();
    }

    pub fn var_delete(&mut self) {
        let keys = self.var_keys();
        if let Some(key) = keys.get(self.var_cursor).cloned() {
            self.var_remove_raw(&key);
            if self.var_cursor > 0 && self.var_cursor >= self.var_keys().len() {
                self.var_cursor -= 1;
            }
            self.var_save_current_tier();
        }
    }

    pub fn var_toggle_secret(&mut self) {
        let keys = self.var_keys();
        if let Some(key) = keys.get(self.var_cursor).cloned() {
            if let Some(var) = self.var_get_mut(&key) {
                var.secret = !var.secret;
                self.var_save_current_tier();
            }
        }
    }

    /// Get active text input for variable editing
    pub fn var_active_input(&mut self) -> Option<&mut crate::text_input::TextInput> {
        match self.var_editing? {
            VarEditTarget::Key => Some(&mut self.var_key_input),
            VarEditTarget::Value => Some(&mut self.var_value_input),
        }
    }

    // ── Private variable helpers ──

    fn var_get_mut(&mut self, key: &str) -> Option<&mut curl_tui_core::types::Variable> {
        match self.var_tier {
            VarTier::Global => self.config.variables.get_mut(key),
            VarTier::Environment => self
                .active_workspace_mut()
                .and_then(|ws| {
                    ws.data
                        .var_environment_idx
                        .and_then(|i| ws.data.environments.get_mut(i))
                })
                .and_then(|e| e.variables.get_mut(key)),
            VarTier::Collection => self
                .active_workspace_mut()
                .and_then(|ws| {
                    ws.data
                        .var_collection_idx
                        .and_then(|i| ws.data.collections.get_mut(i))
                })
                .and_then(|c| c.variables.get_mut(key)),
        }
    }

    fn var_remove_raw(&mut self, key: &str) -> Option<curl_tui_core::types::Variable> {
        match self.var_tier {
            VarTier::Global => self.config.variables.remove(key),
            VarTier::Environment => self
                .active_workspace_mut()
                .and_then(|ws| {
                    ws.data
                        .var_environment_idx
                        .and_then(|i| ws.data.environments.get_mut(i))
                })
                .and_then(|e| e.variables.remove(key)),
            VarTier::Collection => self
                .active_workspace_mut()
                .and_then(|ws| {
                    ws.data
                        .var_collection_idx
                        .and_then(|i| ws.data.collections.get_mut(i))
                })
                .and_then(|c| c.variables.remove(key)),
        }
    }

    fn var_insert_raw(&mut self, key: &str, var: curl_tui_core::types::Variable) {
        match self.var_tier {
            VarTier::Global => {
                self.config.variables.insert(key.to_string(), var);
            }
            VarTier::Environment => {
                if let Some(ws) = self.active_workspace_mut() {
                    if let Some(env) = ws
                        .data
                        .var_environment_idx
                        .and_then(|i| ws.data.environments.get_mut(i))
                    {
                        env.variables.insert(key.to_string(), var);
                    }
                }
            }
            VarTier::Collection => {
                if let Some(ws) = self.active_workspace_mut() {
                    if let Some(col) = ws
                        .data
                        .var_collection_idx
                        .and_then(|i| ws.data.collections.get_mut(i))
                    {
                        col.variables.insert(key.to_string(), var);
                    }
                }
            }
        }
    }

    fn var_save_current_tier(&self) {
        let config_root = config_dir();
        match self.var_tier {
            VarTier::Global => {
                let _ = self.config.save_to(&config_root.join("config.json"));
            }
            VarTier::Environment => {
                if let Some(ws) = self.active_workspace() {
                    if let Some(env) = ws
                        .data
                        .var_environment_idx
                        .and_then(|i| ws.data.environments.get(i))
                    {
                        let env_dir = config_root
                            .join("projects")
                            .join(&ws.data.slug)
                            .join("environments");
                        let _ = curl_tui_core::environment::save_environment(&env_dir, env);
                    }
                }
            }
            VarTier::Collection => {
                if let Some(ws) = self.active_workspace() {
                    if let Some(col) = ws
                        .data
                        .var_collection_idx
                        .and_then(|i| ws.data.collections.get(i))
                    {
                        let collections_dir = config_root
                            .join("projects")
                            .join(&ws.data.slug)
                            .join("collections");
                        let _ = curl_tui_core::collection::save_collection(&collections_dir, col);
                    }
                }
            }
        }
    }
}
