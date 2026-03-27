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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum DigestAlgorithm {
    #[default]
    MD5,
    #[serde(rename = "sha-256")]
    SHA256,
    #[serde(rename = "sha-512-256")]
    SHA512256,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OAuth1SignatureMethod {
    #[serde(rename = "hmac-sha1")]
    HmacSha1,
    #[serde(rename = "hmac-sha256")]
    HmacSha256,
    Plaintext,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuth1AddTo {
    #[default]
    Header,
    Body,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum PkceMethod {
    #[serde(rename = "sha-256")]
    #[default]
    SHA256,
    Plain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "grant_type", rename_all = "snake_case")]
pub enum OAuth2Grant {
    AuthorizationCode {
        auth_url: String,
        token_url: String,
        client_id: String,
        client_secret: String,
    },
    Pkce {
        auth_url: String,
        token_url: String,
        client_id: String,
        client_secret: String,
        #[serde(default)]
        code_challenge_method: PkceMethod,
        #[serde(default)]
        code_verifier: String,
    },
    ClientCredentials {
        token_url: String,
        client_id: String,
        client_secret: String,
    },
    Password {
        token_url: String,
        username: String,
        password: String,
        client_id: String,
        client_secret: String,
    },
}

/// How client credentials are sent to the token endpoint.
/// `BasicHeader` = "Send as Basic Auth header" (serializes as "header").
/// `Body` = "Send client credentials in body" (serializes as "body").
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientAuthentication {
    #[serde(rename = "header")]
    #[default]
    BasicHeader,
    Body,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AwsAddTo {
    #[default]
    Headers,
    Url,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AsapAlgorithm {
    #[default]
    RS256,
    RS384,
    RS512,
    PS256,
    PS384,
    PS512,
    ES256,
    ES384,
    ES512,
}

fn default_oauth1_version() -> String {
    "1.0".to_string()
}

fn default_oauth1_add_to() -> OAuth1AddTo {
    OAuth1AddTo::Header
}

fn default_client_authentication() -> ClientAuthentication {
    ClientAuthentication::BasicHeader
}

fn default_aws_region() -> String {
    "us-east-1".to_string()
}

fn default_aws_add_to() -> AwsAddTo {
    AwsAddTo::Headers
}

fn default_asap_algorithm() -> AsapAlgorithm {
    AsapAlgorithm::RS256
}

fn default_asap_expiry() -> String {
    "3600".to_string()
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
    Digest {
        username: String,
        password: String,
        #[serde(default)]
        realm: String,
        #[serde(default)]
        nonce: String,
        #[serde(default)]
        algorithm: DigestAlgorithm,
        #[serde(default)]
        qop: String,
        #[serde(default)]
        nonce_count: String,
        #[serde(default)]
        client_nonce: String,
        #[serde(default)]
        opaque: String,
    },
    #[serde(rename = "oauth1")]
    OAuth1 {
        signature_method: OAuth1SignatureMethod,
        consumer_key: String,
        consumer_secret: String,
        access_token: String,
        token_secret: String,
        #[serde(default)]
        callback_url: String,
        #[serde(default = "default_oauth1_version")]
        version: String,
        #[serde(default)]
        realm: String,
        #[serde(default)]
        timestamp: String,
        #[serde(default)]
        nonce: String,
        #[serde(default)]
        include_body_hash: bool,
        #[serde(default = "default_oauth1_add_to")]
        add_to: OAuth1AddTo,
    },
    #[serde(rename = "oauth2")]
    OAuth2 {
        grant: OAuth2Grant,
        #[serde(default)]
        token_name: String,
        #[serde(default)]
        callback_url: String,
        #[serde(default)]
        scope: String,
        #[serde(default)]
        state: String,
        #[serde(default = "default_client_authentication")]
        client_authentication: ClientAuthentication,
        #[serde(default)]
        access_token: String,
        #[serde(default)]
        refresh_token: String,
    },
    #[serde(rename = "awsv4")]
    AwsV4 {
        access_key: String,
        secret_key: String,
        #[serde(default = "default_aws_region")]
        region: String,
        #[serde(default)]
        service: String,
        #[serde(default)]
        session_token: String,
        #[serde(default = "default_aws_add_to")]
        add_to: AwsAddTo,
    },
    Asap {
        #[serde(default = "default_asap_algorithm")]
        algorithm: AsapAlgorithm,
        issuer: String,
        audience: String,
        key_id: String,
        private_key: String,
        #[serde(default)]
        subject: String,
        #[serde(default = "default_asap_expiry")]
        expiry: String,
        #[serde(default)]
        additional_claims: String,
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

    /// Restore `active_environment` index from the saved `project.active_environment` name.
    /// Call this after loading environments for a project.
    pub fn restore_active_environment(&mut self) {
        self.active_environment = self
            .project
            .active_environment
            .as_ref()
            .and_then(|name| self.environments.iter().position(|e| &e.name == name));
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

    #[test]
    fn test_restore_active_environment_from_name() {
        let project = Project {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            active_environment: Some("Production".to_string()),
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
        ws.restore_active_environment();
        assert_eq!(ws.active_environment, Some(1));
    }

    #[test]
    fn test_restore_active_environment_none_when_no_match() {
        let project = Project {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            active_environment: Some("Staging".to_string()),
        };
        let mut ws = ProjectWorkspaceData::new(project, "test".to_string());
        ws.environments = vec![Environment {
            id: uuid::Uuid::new_v4(),
            name: "Development".to_string(),
            variables: HashMap::new(),
        }];
        ws.restore_active_environment();
        assert_eq!(ws.active_environment, None);
    }

    #[test]
    fn test_restore_active_environment_none_when_not_set() {
        let project = Project {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            active_environment: None,
        };
        let mut ws = ProjectWorkspaceData::new(project, "test".to_string());
        ws.environments = vec![Environment {
            id: uuid::Uuid::new_v4(),
            name: "Development".to_string(),
            variables: HashMap::new(),
        }];
        ws.restore_active_environment();
        assert_eq!(ws.active_environment, None);
    }

    // --- Supporting enum roundtrip tests ---

    #[test]
    fn test_digest_algorithm_roundtrip() {
        let md5: DigestAlgorithm = serde_json::from_str(r#""MD5""#).unwrap();
        assert_eq!(md5, DigestAlgorithm::MD5);
        let sha256: DigestAlgorithm = serde_json::from_str(r#""sha-256""#).unwrap();
        assert_eq!(sha256, DigestAlgorithm::SHA256);
        let sha512: DigestAlgorithm = serde_json::from_str(r#""sha-512-256""#).unwrap();
        assert_eq!(sha512, DigestAlgorithm::SHA512256);
        assert_eq!(DigestAlgorithm::default(), DigestAlgorithm::MD5);
    }

    #[test]
    fn test_oauth1_signature_method_roundtrip() {
        let hmac1: OAuth1SignatureMethod = serde_json::from_str(r#""hmac-sha1""#).unwrap();
        assert_eq!(hmac1, OAuth1SignatureMethod::HmacSha1);
        let hmac256: OAuth1SignatureMethod = serde_json::from_str(r#""hmac-sha256""#).unwrap();
        assert_eq!(hmac256, OAuth1SignatureMethod::HmacSha256);
        let plain: OAuth1SignatureMethod = serde_json::from_str(r#""Plaintext""#).unwrap();
        assert_eq!(plain, OAuth1SignatureMethod::Plaintext);
    }

    #[test]
    fn test_oauth1_add_to_roundtrip() {
        let header: OAuth1AddTo = serde_json::from_str(r#""header""#).unwrap();
        assert_eq!(header, OAuth1AddTo::Header);
        let body: OAuth1AddTo = serde_json::from_str(r#""body""#).unwrap();
        assert_eq!(body, OAuth1AddTo::Body);
        assert_eq!(OAuth1AddTo::default(), OAuth1AddTo::Header);
    }

    #[test]
    fn test_pkce_method_roundtrip() {
        let sha: PkceMethod = serde_json::from_str(r#""sha-256""#).unwrap();
        assert_eq!(sha, PkceMethod::SHA256);
        let plain: PkceMethod = serde_json::from_str(r#""Plain""#).unwrap();
        assert_eq!(plain, PkceMethod::Plain);
        assert_eq!(PkceMethod::default(), PkceMethod::SHA256);
    }

    #[test]
    fn test_oauth2_grant_authorization_code_roundtrip() {
        let grant = OAuth2Grant::AuthorizationCode {
            auth_url: "https://auth.example.com/authorize".to_string(),
            token_url: "https://auth.example.com/token".to_string(),
            client_id: "my-client".to_string(),
            client_secret: "my-secret".to_string(),
        };
        let json = serde_json::to_string(&grant).unwrap();
        let deserialized: OAuth2Grant = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, grant);
    }

    #[test]
    fn test_oauth2_grant_client_credentials_roundtrip() {
        let grant = OAuth2Grant::ClientCredentials {
            token_url: "https://auth.example.com/token".to_string(),
            client_id: "my-client".to_string(),
            client_secret: "my-secret".to_string(),
        };
        let json = serde_json::to_string(&grant).unwrap();
        let deserialized: OAuth2Grant = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, grant);
    }

    #[test]
    fn test_client_authentication_roundtrip() {
        let header: ClientAuthentication = serde_json::from_str(r#""header""#).unwrap();
        assert_eq!(header, ClientAuthentication::BasicHeader);
        let body: ClientAuthentication = serde_json::from_str(r#""body""#).unwrap();
        assert_eq!(body, ClientAuthentication::Body);
        assert_eq!(
            ClientAuthentication::default(),
            ClientAuthentication::BasicHeader
        );
    }

    #[test]
    fn test_aws_add_to_roundtrip() {
        let headers: AwsAddTo = serde_json::from_str(r#""headers""#).unwrap();
        assert_eq!(headers, AwsAddTo::Headers);
        let url: AwsAddTo = serde_json::from_str(r#""url""#).unwrap();
        assert_eq!(url, AwsAddTo::Url);
        assert_eq!(AwsAddTo::default(), AwsAddTo::Headers);
    }

    #[test]
    fn test_asap_algorithm_roundtrip() {
        let rs256: AsapAlgorithm = serde_json::from_str(r#""RS256""#).unwrap();
        assert_eq!(rs256, AsapAlgorithm::RS256);
        let es512: AsapAlgorithm = serde_json::from_str(r#""ES512""#).unwrap();
        assert_eq!(es512, AsapAlgorithm::ES512);
        assert_eq!(AsapAlgorithm::default(), AsapAlgorithm::RS256);
    }

    // --- Auth variant roundtrip tests ---

    #[test]
    fn test_auth_digest_roundtrip() {
        let auth = Auth::Digest {
            username: "user".to_string(),
            password: "pass".to_string(),
            realm: "example.com".to_string(),
            nonce: "abc123".to_string(),
            algorithm: DigestAlgorithm::SHA256,
            qop: "auth".to_string(),
            nonce_count: "00000001".to_string(),
            client_nonce: "xyz".to_string(),
            opaque: "opq".to_string(),
        };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(json.contains(r#""type":"digest""#));
        let deserialized: Auth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, auth);
    }

    #[test]
    fn test_auth_digest_defaults() {
        let json = r#"{"type":"digest","username":"u","password":"p"}"#;
        let auth: Auth = serde_json::from_str(json).unwrap();
        if let Auth::Digest {
            algorithm,
            realm,
            qop,
            ..
        } = &auth
        {
            assert_eq!(*algorithm, DigestAlgorithm::MD5);
            assert_eq!(realm, "");
            assert_eq!(qop, "");
        } else {
            panic!("Expected Digest variant");
        }
    }

    #[test]
    fn test_auth_oauth1_roundtrip() {
        let auth = Auth::OAuth1 {
            signature_method: OAuth1SignatureMethod::HmacSha1,
            consumer_key: "ck".to_string(),
            consumer_secret: "cs".to_string(),
            access_token: "at".to_string(),
            token_secret: "ts".to_string(),
            callback_url: String::new(),
            version: "1.0".to_string(),
            realm: String::new(),
            timestamp: String::new(),
            nonce: String::new(),
            include_body_hash: false,
            add_to: OAuth1AddTo::Header,
        };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(json.contains(r#""type":"oauth1""#));
        let deserialized: Auth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, auth);
    }

    #[test]
    fn test_auth_oauth1_defaults() {
        let json = r#"{"type":"oauth1","signature_method":"hmac-sha1","consumer_key":"ck","consumer_secret":"cs","access_token":"at","token_secret":"ts"}"#;
        let auth: Auth = serde_json::from_str(json).unwrap();
        if let Auth::OAuth1 {
            version,
            add_to,
            include_body_hash,
            ..
        } = &auth
        {
            assert_eq!(version, "1.0");
            assert_eq!(*add_to, OAuth1AddTo::Header);
            assert!(!include_body_hash);
        } else {
            panic!("Expected OAuth1 variant");
        }
    }

    #[test]
    fn test_auth_oauth2_roundtrip() {
        let auth = Auth::OAuth2 {
            grant: OAuth2Grant::AuthorizationCode {
                auth_url: "https://auth.example.com/authorize".to_string(),
                token_url: "https://auth.example.com/token".to_string(),
                client_id: "cid".to_string(),
                client_secret: "csec".to_string(),
            },
            token_name: String::new(),
            callback_url: String::new(),
            scope: "read write".to_string(),
            state: String::new(),
            client_authentication: ClientAuthentication::BasicHeader,
            access_token: String::new(),
            refresh_token: String::new(),
        };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(json.contains(r#""type":"oauth2""#));
        let deserialized: Auth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, auth);
    }

    #[test]
    fn test_auth_oauth2_pkce_roundtrip() {
        let auth = Auth::OAuth2 {
            grant: OAuth2Grant::Pkce {
                auth_url: "https://auth.example.com/authorize".to_string(),
                token_url: "https://auth.example.com/token".to_string(),
                client_id: "cid".to_string(),
                client_secret: "csec".to_string(),
                code_challenge_method: PkceMethod::SHA256,
                code_verifier: String::new(),
            },
            token_name: String::new(),
            callback_url: String::new(),
            scope: "read write".to_string(),
            state: String::new(),
            client_authentication: ClientAuthentication::BasicHeader,
            access_token: String::new(),
            refresh_token: String::new(),
        };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(json.contains(r#""type":"oauth2""#));
        assert!(json.contains(r#""grant_type":"pkce""#));
        let deserialized: Auth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, auth);
    }

    #[test]
    fn test_auth_oauth2_password_roundtrip() {
        let auth = Auth::OAuth2 {
            grant: OAuth2Grant::Password {
                token_url: "https://auth.example.com/token".to_string(),
                username: "user@example.com".to_string(),
                password: "secret".to_string(),
                client_id: "cid".to_string(),
                client_secret: "csec".to_string(),
            },
            token_name: String::new(),
            callback_url: String::new(),
            scope: "read write".to_string(),
            state: String::new(),
            client_authentication: ClientAuthentication::BasicHeader,
            access_token: String::new(),
            refresh_token: String::new(),
        };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(json.contains(r#""type":"oauth2""#));
        assert!(json.contains(r#""grant_type":"password""#));
        let deserialized: Auth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, auth);
    }

    #[test]
    fn test_auth_awsv4_roundtrip() {
        let auth = Auth::AwsV4 {
            access_key: "AKIA...".to_string(),
            secret_key: "secret".to_string(),
            region: "us-west-2".to_string(),
            service: "s3".to_string(),
            session_token: String::new(),
            add_to: AwsAddTo::Headers,
        };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(json.contains(r#""type":"awsv4""#));
        let deserialized: Auth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, auth);
    }

    #[test]
    fn test_auth_awsv4_defaults() {
        let json = r#"{"type":"awsv4","access_key":"ak","secret_key":"sk"}"#;
        let auth: Auth = serde_json::from_str(json).unwrap();
        if let Auth::AwsV4 { region, add_to, .. } = &auth {
            assert_eq!(region, "us-east-1");
            assert_eq!(*add_to, AwsAddTo::Headers);
        } else {
            panic!("Expected AwsV4 variant");
        }
    }

    #[test]
    fn test_auth_asap_roundtrip() {
        let auth = Auth::Asap {
            algorithm: AsapAlgorithm::RS256,
            issuer: "iss".to_string(),
            audience: "aud".to_string(),
            key_id: "kid".to_string(),
            private_key: "pk".to_string(),
            subject: String::new(),
            expiry: "3600".to_string(),
            additional_claims: String::new(),
        };
        let json = serde_json::to_string(&auth).unwrap();
        assert!(json.contains(r#""type":"asap""#));
        let deserialized: Auth = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, auth);
    }

    #[test]
    fn test_auth_asap_defaults() {
        let json = r#"{"type":"asap","issuer":"i","audience":"a","key_id":"k","private_key":"pk"}"#;
        let auth: Auth = serde_json::from_str(json).unwrap();
        if let Auth::Asap {
            algorithm,
            expiry,
            subject,
            ..
        } = &auth
        {
            assert_eq!(*algorithm, AsapAlgorithm::RS256);
            assert_eq!(expiry, "3600");
            assert_eq!(subject, "");
        } else {
            panic!("Expected Asap variant");
        }
    }

    // --- Backward compatibility tests ---

    #[test]
    fn test_auth_basic_backward_compat() {
        let json = r#"{"type":"basic","username":"admin","password":"secret"}"#;
        let auth: Auth = serde_json::from_str(json).unwrap();
        assert_eq!(
            auth,
            Auth::Basic {
                username: "admin".to_string(),
                password: "secret".to_string(),
            }
        );
    }

    #[test]
    fn test_auth_bearer_backward_compat() {
        let json = r#"{"type":"bearer","token":"my-token"}"#;
        let auth: Auth = serde_json::from_str(json).unwrap();
        assert_eq!(
            auth,
            Auth::Bearer {
                token: "my-token".to_string(),
            }
        );
    }

    #[test]
    fn test_auth_apikey_backward_compat() {
        let json = r#"{"type":"apikey","key":"X-API-Key","value":"abc123","in":"header"}"#;
        let auth: Auth = serde_json::from_str(json).unwrap();
        assert_eq!(
            auth,
            Auth::ApiKey {
                key: "X-API-Key".to_string(),
                value: "abc123".to_string(),
                location: ApiKeyLocation::Header,
            }
        );
    }
}
