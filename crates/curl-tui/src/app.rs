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
#[allow(dead_code)]
pub enum EditField {
    Url,
    HeaderKey(usize),
    HeaderValue(usize),
    ParamKey(usize),
    ParamValue(usize),
    BodyContent,
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
            self.status_message = Some("No collection selected. Create one first.".to_string());
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
        self.status_message = Some("New request created".to_string());
    }

    /// Enter editing mode for a specific field
    pub fn start_editing(&mut self, field: EditField) {
        self.input_mode = InputMode::Editing;
        self.edit_field = Some(field);
    }

    /// Exit editing mode, syncing text input back to request
    pub fn stop_editing(&mut self) {
        if let Some(field) = self.edit_field.take() {
            self.sync_field_to_request(field);
        }
        self.input_mode = InputMode::Normal;
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
            self.status_message = Some("No environments configured".to_string());
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
}
