use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl Method {
    pub const ALL: [Method; 7] = [
        Method::Get,
        Method::Post,
        Method::Put,
        Method::Delete,
        Method::Patch,
        Method::Head,
        Method::Options,
    ];
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Method::Get => write!(f, "GET"),
            Method::Post => write!(f, "POST"),
            Method::Put => write!(f, "PUT"),
            Method::Delete => write!(f, "DELETE"),
            Method::Patch => write!(f, "PATCH"),
            Method::Head => write!(f, "HEAD"),
            Method::Options => write!(f, "OPTIONS"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    pub value: String,
    #[serde(default)]
    pub secret: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Header {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FormField {
    pub key: String,
    pub value: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultipartPart {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Body {
    Json { content: String },
    Text { content: String },
    Form { fields: Vec<FormField> },
    Multipart { parts: Vec<MultipartPart> },
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyLocation {
    Header,
    Query,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Auth {
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
    ApiKey {
        key: String,
        value: String,
        #[serde(rename = "in")]
        location: ApiKeyLocation,
    },
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub id: uuid::Uuid,
    pub name: String,
    pub method: Method,
    pub url: String,
    #[serde(default)]
    pub headers: Vec<Header>,
    #[serde(default)]
    pub params: Vec<Param>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Body>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub id: uuid::Uuid,
    pub name: String,
    #[serde(default)]
    pub variables: HashMap<String, Variable>,
    #[serde(default)]
    pub requests: Vec<Request>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Environment {
    pub id: uuid::Uuid,
    pub name: String,
    #[serde(default)]
    pub variables: HashMap<String, Variable>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: uuid::Uuid,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_environment: Option<String>,
}

/// Timing breakdown for curl responses
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseTiming {
    pub dns_lookup_ms: f64,
    pub tcp_connect_ms: f64,
    pub tls_handshake_ms: f64,
    pub transfer_start_ms: f64,
    pub total_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogHeader {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_template: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogParam {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestLogData {
    pub method: Method,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_template: Option<String>,
    #[serde(default)]
    pub headers: Vec<LogHeader>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_template: Option<String>,
    #[serde(default)]
    pub params: Vec<LogParam>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseLogData {
    pub status_code: u16,
    pub status_text: String,
    #[serde(default)]
    pub headers: Vec<LogHeader>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub body_size_bytes: u64,
    #[serde(default)]
    pub body_truncated: bool,
    pub body_type: String,
    pub time_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestLogEntry {
    pub id: uuid::Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    pub request: RequestLogData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ResponseLogData>,
    pub curl_command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Parsed response from a curl execution
#[derive(Debug, Clone)]
pub struct CurlResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub timing: ResponseTiming,
    pub raw_command: String,
}

/// Core per-project data — business logic fields only (no UI state).
#[derive(Debug, Clone)]
pub struct ProjectWorkspaceData {
    pub project: Project,
    pub slug: String,
    pub collections: Vec<Collection>,
    pub environments: Vec<Environment>,
    pub active_environment: Option<usize>,
    pub selected_collection: Option<usize>,
    pub selected_request: Option<usize>,
    pub current_request: Option<Request>,
    pub last_response: Option<CurlResponse>,
    pub var_collection_idx: Option<usize>,
    pub var_environment_idx: Option<usize>,
}

impl ProjectWorkspaceData {
    pub fn new(project: Project, slug: String) -> Self {
        Self {
            project,
            slug,
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
            var_collection_idx: None,
            var_environment_idx: None,
        }
    }

    /// Sync the `project.active_environment` name field from the current index.
    /// Call this after any mutation of `self.active_environment`.
    pub fn sync_active_environment_name(&mut self) {
        self.project.active_environment = self
            .active_environment
            .and_then(|i| self.environments.get(i))
            .map(|env| env.name.clone());
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_serialization() {
        let method = Method::Post;
        let json = serde_json::to_string(&method).unwrap();
        assert_eq!(json, "\"POST\"");
        let deserialized: Method = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Method::Post);
    }

    #[test]
    fn test_variable_with_secret() {
        let var = Variable {
            value: "my-secret".to_string(),
            secret: true,
        };
        let json = serde_json::to_string(&var).unwrap();
        let deserialized: Variable = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.secret, true);
        assert_eq!(deserialized.value, "my-secret");
    }

    #[test]
    fn test_variable_default_not_secret() {
        let json = r#"{"value": "hello"}"#;
        let var: Variable = serde_json::from_str(json).unwrap();
        assert!(!var.secret);
    }

    #[test]
    fn test_body_json_roundtrip() {
        let body = Body::Json {
            content: r#"{"key": "value"}"#.to_string(),
        };
        let json = serde_json::to_string(&body).unwrap();
        let deserialized: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, body);
    }

    #[test]
    fn test_body_form_roundtrip() {
        let body = Body::Form {
            fields: vec![FormField {
                key: "user".to_string(),
                value: "alice".to_string(),
                enabled: true,
            }],
        };
        let json = serde_json::to_string(&body).unwrap();
        let deserialized: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, body);
    }

    #[test]
    fn test_body_none_roundtrip() {
        let body = Body::None;
        let json = serde_json::to_string(&body).unwrap();
        let deserialized: Body = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, Body::None);
    }

    #[test]
    fn test_auth_bearer_roundtrip() {
        let auth = Auth::Bearer {
            token: "{{api_token}}".to_string(),
        };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(json.contains("bearer"));
        let deserialized: Auth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, auth);
    }

    #[test]
    fn test_auth_api_key_roundtrip() {
        let auth = Auth::ApiKey {
            key: "X-API-Key".to_string(),
            value: "{{key}}".to_string(),
            location: ApiKeyLocation::Header,
        };
        let json = serde_json::to_string(&auth).unwrap();
        let deserialized: Auth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, auth);
    }

    #[test]
    fn test_request_roundtrip() {
        let request = Request {
            id: uuid::Uuid::new_v4(),
            name: "Get Users".to_string(),
            method: Method::Get,
            url: "https://api.example.com/users".to_string(),
            headers: vec![Header {
                key: "Accept".to_string(),
                value: "application/json".to_string(),
                enabled: true,
            }],
            params: vec![Param {
                key: "page".to_string(),
                value: "1".to_string(),
                enabled: true,
            }],
            body: None,
            auth: Some(Auth::None),
        };
        let json = serde_json::to_string_pretty(&request).unwrap();
        let deserialized: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Get Users");
        assert_eq!(deserialized.method, Method::Get);
        assert_eq!(deserialized.headers.len(), 1);
        assert_eq!(deserialized.params.len(), 1);
    }

    #[test]
    fn test_collection_roundtrip() {
        let collection = Collection {
            id: uuid::Uuid::new_v4(),
            name: "My API".to_string(),
            variables: std::collections::HashMap::new(),
            requests: vec![],
        };
        let json = serde_json::to_string(&collection).unwrap();
        let deserialized: Collection = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "My API");
    }

    #[test]
    fn test_environment_roundtrip() {
        let mut vars = std::collections::HashMap::new();
        vars.insert(
            "base_url".to_string(),
            Variable {
                value: "http://localhost:3000".to_string(),
                secret: false,
            },
        );
        let env = Environment {
            id: uuid::Uuid::new_v4(),
            name: "Development".to_string(),
            variables: vars,
        };
        let json = serde_json::to_string(&env).unwrap();
        let deserialized: Environment = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Development");
        assert!(deserialized.variables.contains_key("base_url"));
    }

    #[test]
    fn test_project_roundtrip() {
        let project = Project {
            id: uuid::Uuid::new_v4(),
            name: "My API".to_string(),
            active_environment: Some("dev".to_string()),
        };
        let json = serde_json::to_string(&project).unwrap();
        let deserialized: Project = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "My API");
        assert_eq!(deserialized.active_environment, Some("dev".to_string()));
    }

    #[test]
    fn test_project_without_active_env() {
        let json = r#"{"id":"00000000-0000-0000-0000-000000000000","name":"Test"}"#;
        let project: Project = serde_json::from_str(json).unwrap();
        assert!(project.active_environment.is_none());
    }

    #[test]
    fn test_request_log_entry_roundtrip() {
        let entry = RequestLogEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            project: Some("my-api".to_string()),
            collection: Some("auth-endpoints".to_string()),
            request: RequestLogData {
                method: Method::Post,
                url: "https://api.example.com/login".to_string(),
                url_template: Some("https://{{base_url}}/login".to_string()),
                headers: vec![
                    LogHeader {
                        name: "Content-Type".to_string(),
                        value: "application/json".to_string(),
                        value_template: None,
                    },
                    LogHeader {
                        name: "Authorization".to_string(),
                        value: "[REDACTED]".to_string(),
                        value_template: Some("Bearer {{api_token}}".to_string()),
                    },
                ],
                body: Some(r#"{"user": "test"}"#.to_string()),
                body_template: Some(r#"{"user": "{{username}}"}"#.to_string()),
                params: vec![LogParam {
                    name: "debug".to_string(),
                    value: "true".to_string(),
                }],
            },
            response: Some(ResponseLogData {
                status_code: 200,
                status_text: "OK".to_string(),
                headers: vec![LogHeader {
                    name: "Content-Type".to_string(),
                    value: "application/json".to_string(),
                    value_template: None,
                }],
                body: Some(r#"{"token": "[REDACTED]"}"#.to_string()),
                body_size_bytes: 1024,
                body_truncated: false,
                body_type: "text".to_string(),
                time_ms: 342,
            }),
            curl_command: "curl -X POST https://api.example.com/login".to_string(),
            error: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: RequestLogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.request.method, Method::Post);
        assert_eq!(deserialized.response.unwrap().status_code, 200);
        assert_eq!(deserialized.request.headers.len(), 2);
        assert_eq!(
            deserialized.request.headers[1].value_template,
            Some("Bearer {{api_token}}".to_string())
        );
    }

    #[test]
    fn test_request_log_entry_minimal() {
        let entry = RequestLogEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            project: None,
            collection: None,
            request: RequestLogData {
                method: Method::Get,
                url: "https://example.com".to_string(),
                url_template: None,
                headers: vec![],
                body: None,
                body_template: None,
                params: vec![],
            },
            response: None,
            curl_command: "curl https://example.com".to_string(),
            error: Some("Connection refused".to_string()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: RequestLogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.error, Some("Connection refused".to_string()));
        assert!(deserialized.response.is_none());
    }

    fn make_workspace_with_envs() -> ProjectWorkspaceData {
        let project = Project {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            active_environment: None,
        };
        let mut ws = ProjectWorkspaceData::new(project, "test".to_string());
        ws.environments = vec![
            Environment {
                id: uuid::Uuid::new_v4(),
                name: "Development".to_string(),
                variables: HashMap::new(),
            },
            Environment {
                id: uuid::Uuid::new_v4(),
                name: "Production".to_string(),
                variables: HashMap::new(),
            },
        ];
        ws
    }

    #[test]
    fn test_sync_sets_name_from_index() {
        let mut ws = make_workspace_with_envs();
        ws.active_environment = Some(1);
        ws.sync_active_environment_name();
        assert_eq!(
            ws.project.active_environment,
            Some("Production".to_string())
        );
    }

    #[test]
    fn test_sync_clears_name_when_none() {
        let mut ws = make_workspace_with_envs();
        ws.active_environment = None;
        ws.sync_active_environment_name();
        assert_eq!(ws.project.active_environment, None);
    }

    #[test]
    fn test_sync_clears_name_when_index_out_of_bounds() {
        let mut ws = make_workspace_with_envs();
        ws.active_environment = Some(99);
        ws.sync_active_environment_name();
        assert_eq!(ws.project.active_environment, None);
    }
}
