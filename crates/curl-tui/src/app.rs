use curl_tui_core::command::CurlCommandBuilder;
use curl_tui_core::config::{config_dir, AppConfig};
use curl_tui_core::history::append_entry_redacted;
use curl_tui_core::types::{
    Auth, Body, Collection, CurlResponse, Environment, HistoryEntry, Method, Request,
};
use curl_tui_core::variable::FileVariableResolver;

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
