use curl_tui_core::command::CurlCommandBuilder;
use curl_tui_core::config::{config_dir, AppConfig};
use curl_tui_core::history::append_entry_redacted;
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
    None,
}

pub struct App {
    pub config: AppConfig,
    pub collections: Vec<Collection>,
    pub environments: Vec<Environment>,
    pub active_environment: Option<usize>,
    pub selected_collection: Option<usize>,
    pub selected_request: Option<usize>,
    pub current_request: Option<Request>,
    pub last_response: Option<CurlResponse>,
    pub active_pane: Pane,
    pub request_tab: RequestTab,
    pub response_tab: ResponseTab,
    pub pane_visible: [bool; 3], // [collections, request, response]
    pub should_quit: bool,
    pub show_help: bool,
    pub secrets_revealed: bool,
    pub status_message: Option<String>,
    pub input_mode: InputMode,
    pub edit_field: Option<EditField>,
    pub url_input: crate::text_input::TextInput,
    pub body_input: crate::text_input::TextInput,
    pub header_key_inputs: Vec<crate::text_input::TextInput>,
    pub header_value_inputs: Vec<crate::text_input::TextInput>,
    pub param_key_inputs: Vec<crate::text_input::TextInput>,
    pub param_value_inputs: Vec<crate::text_input::TextInput>,
    pub name_input: crate::text_input::TextInput,
    // Variables overlay
    pub show_variables: bool,
    pub var_tier: VarTier,
    pub var_cursor: usize,
    pub var_editing: Option<VarEditTarget>,
    pub var_key_input: crate::text_input::TextInput,
    pub var_value_input: crate::text_input::TextInput,
    pub collection_scroll: usize,
    pub response_scroll: usize,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            collections: Vec::new(),
            environments: Vec::new(),
            active_environment: None,
            selected_collection: None,
            selected_request: None,
            current_request: Some(Request {
                id: uuid::Uuid::new_v4(),
                name: "New Request".to_string(),
                method: Method::Get,
                url: String::new(),
                headers: Vec::new(),
                params: Vec::new(),
                body: None,
                auth: None,
            }),
            last_response: None,
            active_pane: Pane::Request,
            request_tab: RequestTab::Headers,
            response_tab: ResponseTab::Body,
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
            show_variables: false,
            var_tier: VarTier::Global,
            var_cursor: 0,
            var_editing: None,
            var_key_input: crate::text_input::TextInput::new(""),
            var_value_input: crate::text_input::TextInput::new(""),
            collection_scroll: 0,
            response_scroll: 0,
        }
    }

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

    pub async fn send_request(&mut self) {
        let Some(request) = &self.current_request else {
            self.status_message = Some("No request to send".to_string());
            return;
        };

        if request.url.is_empty() {
            self.status_message = Some("URL is empty".to_string());
            return;
        }

        // Build variable resolver
        let global_vars = self.config.variables.clone();
        let env_vars = self
            .active_environment
            .and_then(|i| self.environments.get(i))
            .map(|e| e.variables.clone());
        let col_vars = self
            .selected_collection
            .and_then(|i| self.collections.get(i))
            .map(|c| c.variables.clone());

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
            .timeout(self.config.default_timeout);

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
                // Truncate body if needed
                let max_size = self.config.max_response_body_size_bytes as usize;
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

                // Log to history
                let history_path = config_dir().join("history.jsonl");
                let method = request.method;
                let request_name = request.name.clone();
                let entry = HistoryEntry {
                    id: uuid::Uuid::new_v4(),
                    timestamp: chrono::Utc::now(),
                    collection_id: self
                        .selected_collection
                        .and_then(|i| self.collections.get(i))
                        .map(|c| c.id),
                    request_name,
                    method,
                    url: resolved_url,
                    status_code: Some(resp.status_code),
                    duration_ms: Some(resp.timing.total_ms as u64),
                    environment: self
                        .active_environment
                        .and_then(|i| self.environments.get(i))
                        .map(|e| e.name.clone()),
                };
                let _ = append_entry_redacted(&history_path, &entry, &secrets);

                self.last_response = Some(resp);
            }
            Err(e) => {
                self.status_message = Some(format!("Error: {}", e));
            }
        }
    }

    pub fn save_current_request(&mut self) {
        let Some(request) = &self.current_request else {
            self.status_message = Some("No request to save".to_string());
            return;
        };

        if let Some(col_idx) = self.selected_collection {
            if let Some(collection) = self.collections.get_mut(col_idx) {
                // Update existing or add new
                if let Some(req_idx) = self.selected_request {
                    if let Some(existing) = collection.requests.get_mut(req_idx) {
                        *existing = request.clone();
                    }
                } else {
                    collection.requests.push(request.clone());
                    self.selected_request = Some(collection.requests.len() - 1);
                }
                let collections_dir = config_dir().join("collections");
                match curl_tui_core::collection::save_collection(&collections_dir, collection) {
                    Ok(_) => self.status_message = Some("Saved!".to_string()),
                    Err(e) => self.status_message = Some(format!("Save error: {}", e)),
                }
            }
        } else {
            // No collection selected — prompt for a new collection name
            self.name_input.set_content("My Collection");
            self.start_editing(EditField::NewCollectionName);
            self.status_message =
                Some("Name your collection, then press Enter to save".to_string());
        }
    }

    pub fn create_new_collection(&mut self) {
        let collection = Collection {
            id: uuid::Uuid::new_v4(),
            name: "New Collection".to_string(),
            variables: std::collections::HashMap::new(),
            requests: Vec::new(),
        };
        let collections_dir = config_dir().join("collections");
        match curl_tui_core::collection::save_collection(&collections_dir, &collection) {
            Ok(_) => {
                self.collections.push(collection);
                let idx = self.collections.len() - 1;
                self.selected_collection = Some(idx);
                self.selected_request = None;
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
        let Some(col_idx) = self.selected_collection else {
            self.status_message = Some("Nothing selected".to_string());
            return;
        };

        if let Some(req_idx) = self.selected_request {
            // Delete a request from the collection
            if let Some(collection) = self.collections.get_mut(col_idx) {
                if req_idx < collection.requests.len() {
                    let name = collection.requests[req_idx].name.clone();
                    collection.requests.remove(req_idx);

                    // Save collection to disk
                    let collections_dir = config_dir().join("collections");
                    let _ =
                        curl_tui_core::collection::save_collection(&collections_dir, collection);

                    // Adjust selection
                    if collection.requests.is_empty() {
                        self.selected_request = None;
                    } else if req_idx >= collection.requests.len() {
                        self.selected_request = Some(collection.requests.len() - 1);
                    }

                    // Clear current request if it was the deleted one
                    if self
                        .current_request
                        .as_ref()
                        .is_some_and(|r| r.name == name)
                    {
                        self.current_request = None;
                        self.last_response = None;
                    }

                    self.status_message = Some(format!("Deleted request '{}'", name));
                }
            }
        } else {
            // Delete the entire collection
            if let Some(collection) = self.collections.get(col_idx) {
                let name = collection.name.clone();
                let slug = curl_tui_core::collection::slugify(&name);
                let path = config_dir()
                    .join("collections")
                    .join(format!("{}.json", slug));
                if path.exists() {
                    let _ = std::fs::remove_file(&path);
                }

                self.collections.remove(col_idx);

                // Adjust selection
                if self.collections.is_empty() {
                    self.selected_collection = None;
                } else if col_idx >= self.collections.len() {
                    self.selected_collection = Some(self.collections.len() - 1);
                }
                self.selected_request = None;

                self.status_message = Some(format!("Deleted collection '{}'", name));
            }
        }
    }

    pub fn new_request(&mut self) {
        self.current_request = Some(Request {
            id: uuid::Uuid::new_v4(),
            name: "New Request".to_string(),
            method: Method::Get,
            url: String::new(),
            headers: Vec::new(),
            params: Vec::new(),
            body: None,
            auth: None,
        });
        self.selected_request = None;
        self.last_response = None;
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

        let request = match &self.current_request {
            Some(r) => r.clone(),
            None => return,
        };

        let new_collection = Collection {
            id: uuid::Uuid::new_v4(),
            name: name.clone(),
            variables: std::collections::HashMap::new(),
            requests: vec![request],
        };
        let collections_dir = config_dir().join("collections");
        match curl_tui_core::collection::save_collection(&collections_dir, &new_collection) {
            Ok(_) => {
                self.collections.push(new_collection);
                let col_idx = self.collections.len() - 1;
                self.selected_collection = Some(col_idx);
                self.selected_request = Some(0);
                self.status_message = Some(format!("Created '{}' and saved!", name));
            }
            Err(e) => self.status_message = Some(format!("Save error: {}", e)),
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
            | EditField::NewCollectionName => Some(&mut self.name_input),
        }
    }

    /// Sync the text input content back to the current request
    fn sync_field_to_request(&mut self, field: EditField) {
        let Some(request) = &mut self.current_request else {
            return;
        };
        match field {
            EditField::Url => {
                request.url = self.url_input.content().to_string();
            }
            EditField::BodyContent => {
                let content = self.body_input.content().to_string();
                if content.is_empty() {
                    request.body = Option::None;
                } else {
                    request.body = Some(curl_tui_core::types::Body::Json { content });
                }
            }
            EditField::HeaderKey(i) => {
                if let Some(header) = request.headers.get_mut(i) {
                    header.key = self.header_key_inputs[i].content().to_string();
                }
            }
            EditField::HeaderValue(i) => {
                if let Some(header) = request.headers.get_mut(i) {
                    header.value = self.header_value_inputs[i].content().to_string();
                }
            }
            EditField::ParamKey(i) => {
                if let Some(param) = request.params.get_mut(i) {
                    param.key = self.param_key_inputs[i].content().to_string();
                }
            }
            EditField::ParamValue(i) => {
                if let Some(param) = request.params.get_mut(i) {
                    param.value = self.param_value_inputs[i].content().to_string();
                }
            }
            EditField::RequestName => {
                let name = self.name_input.content().to_string();
                if !name.is_empty() {
                    request.name = name.clone();
                    // Auto-save the renamed request to its collection
                    if let Some(col_idx) = self.selected_collection {
                        if let Some(req_idx) = self.selected_request {
                            if let Some(collection) = self.collections.get_mut(col_idx) {
                                if let Some(existing) = collection.requests.get_mut(req_idx) {
                                    existing.name = name;
                                }
                                let collections_dir = config_dir().join("collections");
                                match curl_tui_core::collection::save_collection(
                                    &collections_dir,
                                    collection,
                                ) {
                                    Ok(_) => {
                                        self.status_message = Some("Renamed and saved!".to_string())
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
            EditField::CollectionName(col_idx) => {
                let name = self.name_input.content().to_string();
                if !name.is_empty() {
                    if let Some(collection) = self.collections.get_mut(col_idx) {
                        collection.name = name;
                        let collections_dir = config_dir().join("collections");
                        let _ = curl_tui_core::collection::save_collection(
                            &collections_dir,
                            collection,
                        );
                    }
                }
            }
            EditField::EnvironmentName(env_idx) => {
                let name = self.name_input.content().to_string();
                if !name.is_empty() {
                    if let Some(env) = self.environments.get_mut(env_idx) {
                        env.name = name.clone();
                        let config_root = config_dir();
                        match curl_tui_core::environment::save_environment(
                            &config_root.join("environments"),
                            env,
                        ) {
                            Ok(_) => {
                                self.status_message = Some(format!("Environment '{}' saved!", name))
                            }
                            Err(e) => self.status_message = Some(format!("Save error: {}", e)),
                        }
                    }
                }
            }
            EditField::NewCollectionName => {
                // Handled separately in save flow — sync is a no-op here
            }
        }
    }

    /// Load a request's fields into the text inputs
    pub fn load_request_into_inputs(&mut self) {
        if let Some(request) = &self.current_request {
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
                if let Some(col_idx) = self.selected_collection {
                    if let Some(req_idx) = self.selected_request {
                        // Rename a request — clone data first to avoid borrow issues
                        let req_clone = self
                            .collections
                            .get(col_idx)
                            .and_then(|c| c.requests.get(req_idx))
                            .cloned();
                        if let Some(req) = req_clone {
                            self.name_input.set_content(&req.name);
                            self.current_request = Some(req);
                            self.load_request_into_inputs();
                            self.start_editing(EditField::RequestName);
                            self.status_message = Some("Rename request".to_string());
                        }
                    } else {
                        // Rename a collection
                        let col_name = self.collections.get(col_idx).map(|c| c.name.clone());
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
                if let Some(req) = &self.current_request {
                    self.name_input.set_content(&req.name);
                    self.start_editing(EditField::RequestName);
                    self.status_message = Some("Rename request".to_string());
                }
            }
            _ => {}
        }
    }

    /// Add a new empty header to the current request
    pub fn add_header(&mut self) {
        if let Some(request) = &mut self.current_request {
            request.headers.push(curl_tui_core::types::Header {
                key: String::new(),
                value: String::new(),
                enabled: true,
            });
            self.header_key_inputs
                .push(crate::text_input::TextInput::default());
            self.header_value_inputs
                .push(crate::text_input::TextInput::default());
            let idx = request.headers.len() - 1;
            self.start_editing(EditField::HeaderKey(idx));
        }
    }

    /// Add a new empty param to the current request
    pub fn add_param(&mut self) {
        if let Some(request) = &mut self.current_request {
            request.params.push(curl_tui_core::types::Param {
                key: String::new(),
                value: String::new(),
                enabled: true,
            });
            self.param_key_inputs
                .push(crate::text_input::TextInput::default());
            self.param_value_inputs
                .push(crate::text_input::TextInput::default());
            let idx = request.params.len() - 1;
            self.start_editing(EditField::ParamKey(idx));
        }
    }

    /// Handle Enter in Normal mode based on active pane
    pub fn handle_enter(&mut self) {
        match self.active_pane {
            Pane::Collections => {
                // Load the selected request
                if let Some(col_idx) = self.selected_collection {
                    if let Some(collection) = self.collections.get(col_idx) {
                        if let Some(req_idx) = self.selected_request {
                            if let Some(req) = collection.requests.get(req_idx) {
                                let cloned = req.clone();
                                let name = cloned.name.clone();
                                self.current_request = Some(cloned);
                                self.load_request_into_inputs();
                                self.active_pane = Pane::Request;
                                self.status_message = Some(format!("Loaded: {}", name));
                            }
                        }
                    }
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
                if self.response_scroll > 0 {
                    self.response_scroll -= 1;
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
                self.response_scroll = self.response_scroll.saturating_add(1);
            }
        }
    }

    /// Calculate the flat index of the current collection cursor position
    fn collection_cursor_flat_index(&self) -> usize {
        let mut idx = 0;
        for (col_idx, col) in self.collections.iter().enumerate() {
            if Some(col_idx) == self.selected_collection && self.selected_request.is_none() {
                return idx;
            }
            idx += 1; // collection row
            for (req_idx, _) in col.requests.iter().enumerate() {
                if Some(col_idx) == self.selected_collection
                    && Some(req_idx) == self.selected_request
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
        if cursor < self.collection_scroll {
            self.collection_scroll = cursor;
        } else if cursor >= self.collection_scroll + visible_height {
            self.collection_scroll = cursor - visible_height + 1;
        }
    }

    /// Move collection cursor up through the flat list of collections and their requests
    fn move_collection_cursor_up(&mut self) {
        if let Some(req_idx) = self.selected_request {
            if req_idx > 0 {
                self.selected_request = Some(req_idx - 1);
            } else {
                // Move back to collection level
                self.selected_request = Option::None;
            }
        } else if let Some(col_idx) = self.selected_collection {
            if col_idx > 0 {
                self.selected_collection = Some(col_idx - 1);
                // Select last request of previous collection
                if let Some(col) = self.collections.get(col_idx - 1) {
                    if !col.requests.is_empty() {
                        self.selected_request = Some(col.requests.len() - 1);
                    }
                }
            }
        } else if !self.collections.is_empty() {
            self.selected_collection = Some(0);
        }
    }

    /// Move collection cursor down through the flat list
    fn move_collection_cursor_down(&mut self) {
        if let Some(col_idx) = self.selected_collection {
            if let Some(collection) = self.collections.get(col_idx) {
                if let Some(req_idx) = self.selected_request {
                    if req_idx + 1 < collection.requests.len() {
                        self.selected_request = Some(req_idx + 1);
                    } else if col_idx + 1 < self.collections.len() {
                        // Move to next collection
                        self.selected_collection = Some(col_idx + 1);
                        self.selected_request = Option::None;
                    }
                } else if !collection.requests.is_empty() {
                    self.selected_request = Some(0);
                } else if col_idx + 1 < self.collections.len() {
                    self.selected_collection = Some(col_idx + 1);
                }
            }
        } else if !self.collections.is_empty() {
            self.selected_collection = Some(0);
        }
    }

    /// Switch to next request tab
    pub fn next_request_tab(&mut self) {
        self.request_tab = match self.request_tab {
            RequestTab::Headers => RequestTab::Body,
            RequestTab::Body => RequestTab::Auth,
            RequestTab::Auth => RequestTab::Params,
            RequestTab::Params => RequestTab::Headers,
        };
    }

    /// Switch to previous request tab
    pub fn prev_request_tab(&mut self) {
        self.request_tab = match self.request_tab {
            RequestTab::Headers => RequestTab::Params,
            RequestTab::Body => RequestTab::Headers,
            RequestTab::Auth => RequestTab::Body,
            RequestTab::Params => RequestTab::Auth,
        };
    }

    /// Switch to next response tab
    pub fn next_response_tab(&mut self) {
        self.response_tab = match self.response_tab {
            ResponseTab::Body => ResponseTab::Headers,
            ResponseTab::Headers => ResponseTab::Timing,
            ResponseTab::Timing => ResponseTab::Body,
        };
    }

    /// Switch to previous response tab
    pub fn prev_response_tab(&mut self) {
        self.response_tab = match self.response_tab {
            ResponseTab::Body => ResponseTab::Timing,
            ResponseTab::Headers => ResponseTab::Body,
            ResponseTab::Timing => ResponseTab::Headers,
        };
    }

    pub fn cycle_environment(&mut self) {
        if self.environments.is_empty() {
            // No environments — create one and prompt for name
            self.create_new_environment();
            return;
        }
        self.active_environment = Some(match self.active_environment {
            Some(i) => (i + 1) % self.environments.len(),
            None => 0,
        });
        if let Some(env) = self
            .active_environment
            .and_then(|i| self.environments.get(i))
        {
            self.status_message = Some(format!("Environment: {}", env.name));
        }
    }

    pub fn create_new_environment(&mut self) {
        let name = if self.environments.is_empty() {
            "Development"
        } else {
            "New Environment"
        };
        let env = Environment {
            id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            variables: std::collections::HashMap::new(),
        };
        let config_root = config_dir();
        match curl_tui_core::environment::save_environment(&config_root.join("environments"), &env)
        {
            Ok(_) => {
                self.environments.push(env);
                let idx = self.environments.len() - 1;
                self.active_environment = Some(idx);
                // Prompt to rename it
                self.name_input.set_content(name);
                self.start_editing(EditField::EnvironmentName(idx));
                self.status_message = Some("Name your environment, then press Enter".to_string());
            }
            Err(e) => {
                self.status_message = Some(format!("Error creating environment: {}", e));
            }
        }
    }

    pub fn delete_active_environment(&mut self) {
        let Some(idx) = self.active_environment else {
            self.status_message = Some("No environment selected".to_string());
            return;
        };
        let Some(env) = self.environments.get(idx) else {
            return;
        };

        // Delete the file from disk
        let config_root = config_dir();
        let slug = curl_tui_core::collection::slugify(&env.name);
        let path = config_root
            .join("environments")
            .join(format!("{}.json", slug));
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }

        let name = env.name.clone();
        self.environments.remove(idx);

        // Adjust active_environment
        if self.environments.is_empty() {
            self.active_environment = None;
        } else if idx >= self.environments.len() {
            self.active_environment = Some(self.environments.len() - 1);
        }

        self.status_message = Some(format!("Deleted environment '{}'", name));
    }

    // ── Variables overlay ──────────────────────────────────────

    /// Get the sorted keys for the current variable tier
    pub fn var_keys(&self) -> Vec<String> {
        let map = match self.var_tier {
            VarTier::Global => &self.config.variables,
            VarTier::Environment => {
                if let Some(env) = self
                    .active_environment
                    .and_then(|i| self.environments.get(i))
                {
                    &env.variables
                } else {
                    return Vec::new();
                }
            }
            VarTier::Collection => {
                if let Some(col) = self
                    .selected_collection
                    .and_then(|i| self.collections.get(i))
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
                .active_environment
                .and_then(|i| self.environments.get(i))
                .and_then(|e| e.variables.get(key)),
            VarTier::Collection => self
                .selected_collection
                .and_then(|i| self.collections.get(i))
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
                .active_environment
                .and_then(|i| self.environments.get_mut(i))
                .and_then(|e| e.variables.get_mut(key)),
            VarTier::Collection => self
                .selected_collection
                .and_then(|i| self.collections.get_mut(i))
                .and_then(|c| c.variables.get_mut(key)),
        }
    }

    fn var_remove_raw(&mut self, key: &str) -> Option<curl_tui_core::types::Variable> {
        match self.var_tier {
            VarTier::Global => self.config.variables.remove(key),
            VarTier::Environment => self
                .active_environment
                .and_then(|i| self.environments.get_mut(i))
                .and_then(|e| e.variables.remove(key)),
            VarTier::Collection => self
                .selected_collection
                .and_then(|i| self.collections.get_mut(i))
                .and_then(|c| c.variables.remove(key)),
        }
    }

    fn var_insert_raw(&mut self, key: &str, var: curl_tui_core::types::Variable) {
        match self.var_tier {
            VarTier::Global => {
                self.config.variables.insert(key.to_string(), var);
            }
            VarTier::Environment => {
                if let Some(env) = self
                    .active_environment
                    .and_then(|i| self.environments.get_mut(i))
                {
                    env.variables.insert(key.to_string(), var);
                }
            }
            VarTier::Collection => {
                if let Some(col) = self
                    .selected_collection
                    .and_then(|i| self.collections.get_mut(i))
                {
                    col.variables.insert(key.to_string(), var);
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
                if let Some(env) = self
                    .active_environment
                    .and_then(|i| self.environments.get(i))
                {
                    let _ = curl_tui_core::environment::save_environment(
                        &config_root.join("environments"),
                        env,
                    );
                }
            }
            VarTier::Collection => {
                if let Some(col) = self
                    .selected_collection
                    .and_then(|i| self.collections.get(i))
                {
                    let _ = curl_tui_core::collection::save_collection(
                        &config_root.join("collections"),
                        col,
                    );
                }
            }
        }
    }
}
