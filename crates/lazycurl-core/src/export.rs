use crate::collection::slugify;
use crate::command::CurlCommandBuilder;
use crate::config::config_dir;
use crate::types::{ApiKeyLocation, Auth, Body, Request};

use std::path::PathBuf;

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Curl,
    PostmanV21,
    OpenApi3,
}

impl ExportFormat {
    /// Formats available when exporting a single request.
    pub fn request_formats() -> &'static [ExportFormat] {
        &[ExportFormat::Curl, ExportFormat::PostmanV21, ExportFormat::OpenApi3]
    }

    /// Formats available when exporting a collection (curl excluded).
    pub fn collection_formats() -> &'static [ExportFormat] {
        &[ExportFormat::PostmanV21, ExportFormat::OpenApi3]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ExportFormat::Curl => "cURL command",
            ExportFormat::PostmanV21 => "Postman Collection v2.1",
            ExportFormat::OpenApi3 => "OpenAPI 3.0",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            ExportFormat::Curl => unreachable!("curl goes to clipboard, not file"),
            ExportFormat::PostmanV21 => ".postman_collection.json",
            ExportFormat::OpenApi3 => ".openapi.json",
        }
    }
}

/// Returns the exports directory path.
pub fn exports_dir() -> PathBuf {
    config_dir().join("exports")
}

/// Build a complete curl command string from a Request.
pub fn export_curl(request: &Request, secrets: &[String]) -> String {
    let mut builder = CurlCommandBuilder::new(&request.url).method(request.method);

    for header in &request.headers {
        if header.enabled {
            builder = builder.header(&header.key, &header.value);
        }
    }

    for param in &request.params {
        if param.enabled {
            builder = builder.query_param(&param.key, &param.value);
        }
    }

    if let Some(body) = &request.body {
        match body {
            Body::Json { content } => {
                builder = builder.body_json(content);
            }
            Body::Text { content } => {
                builder = builder.body_text(content);
            }
            Body::Form { fields } => {
                for field in fields {
                    if field.enabled {
                        builder = builder.form_field(&field.key, &field.value);
                    }
                }
            }
            Body::Multipart { parts } => {
                for part in parts {
                    if let Some(value) = &part.value {
                        builder = builder.multipart_field(&part.name, value);
                    }
                    if let Some(path) = &part.file_path {
                        builder = builder.multipart_file(&part.name, path);
                    }
                }
            }
            Body::None => {}
        }
    }

    if let Some(auth) = &request.auth {
        match auth {
            Auth::Bearer { token } => {
                builder = builder.header("Authorization", &format!("Bearer {}", token));
            }
            Auth::Basic { username, password } => {
                builder = builder.basic_auth(username, password);
            }
            Auth::ApiKey { key, value, location } => match location {
                ApiKeyLocation::Header => {
                    builder = builder.header(key, value);
                }
                ApiKeyLocation::Query => {
                    builder = builder.query_param(key, value);
                }
            },
            Auth::None => {}
        }
    }

    let cmd = builder.build();
    cmd.to_display_string(secrets)
}

/// Generate a timestamped filename for an export.
pub fn export_filename(name: &str, format: ExportFormat) -> String {
    let slug = slugify(name);
    let now = chrono::Utc::now().format("%Y-%m-%d-%H%M%S");
    format!("{}-{}{}", slug, now, format.file_extension())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FormField, Header, Method, Param};

    fn make_request() -> Request {
        Request {
            id: uuid::Uuid::new_v4(),
            name: "Get Users".to_string(),
            method: Method::Get,
            url: "https://api.example.com/users".to_string(),
            headers: vec![
                Header { key: "Accept".to_string(), value: "application/json".to_string(), enabled: true },
                Header { key: "X-Debug".to_string(), value: "true".to_string(), enabled: false },
            ],
            params: vec![
                Param { key: "page".to_string(), value: "1".to_string(), enabled: true },
            ],
            body: None,
            auth: Some(Auth::Bearer { token: "my-secret-token".to_string() }),
        }
    }

    #[test]
    fn test_export_curl_includes_headers_and_params() {
        let req = make_request();
        let curl = export_curl(&req, &[]);
        assert!(curl.contains("-H"));
        assert!(curl.contains("Accept: application/json"));
        assert!(!curl.contains("X-Debug"));
        assert!(curl.contains("page=1"));
        assert!(curl.contains("Authorization: Bearer my-secret-token"));
    }

    #[test]
    fn test_export_curl_redacts_secrets() {
        let req = make_request();
        let curl = export_curl(&req, &["my-secret-token".to_string()]);
        assert!(!curl.contains("my-secret-token"));
        assert!(curl.contains("••••••"));
    }

    #[test]
    fn test_export_curl_json_body() {
        let req = Request {
            id: uuid::Uuid::new_v4(),
            name: "Create User".to_string(),
            method: Method::Post,
            url: "https://api.example.com/users".to_string(),
            headers: vec![],
            params: vec![],
            body: Some(Body::Json { content: r#"{"name":"Alice"}"#.to_string() }),
            auth: None,
        };
        let curl = export_curl(&req, &[]);
        assert!(curl.contains("-d"));
        assert!(curl.contains(r#"{"name":"Alice"}"#));
        assert!(curl.contains("-X"));
        assert!(curl.contains("POST"));
    }

    #[test]
    fn test_export_curl_form_body() {
        let req = Request {
            id: uuid::Uuid::new_v4(),
            name: "Login".to_string(),
            method: Method::Post,
            url: "https://example.com/login".to_string(),
            headers: vec![],
            params: vec![],
            body: Some(Body::Form {
                fields: vec![
                    FormField { key: "user".to_string(), value: "alice".to_string(), enabled: true },
                    FormField { key: "disabled".to_string(), value: "skip".to_string(), enabled: false },
                ],
            }),
            auth: None,
        };
        let curl = export_curl(&req, &[]);
        assert!(curl.contains("--data-urlencode"));
        assert!(curl.contains("user=alice"));
        assert!(!curl.contains("disabled=skip"));
    }

    #[test]
    fn test_export_curl_basic_auth() {
        let req = Request {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            method: Method::Get,
            url: "https://example.com".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: Some(Auth::Basic { username: "user".to_string(), password: "pass".to_string() }),
        };
        let curl = export_curl(&req, &[]);
        assert!(curl.contains("-u"));
        assert!(curl.contains("user:pass"));
    }

    #[test]
    fn test_export_curl_apikey_header() {
        let req = Request {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            method: Method::Get,
            url: "https://example.com".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: Some(Auth::ApiKey {
                key: "X-API-Key".to_string(),
                value: "abc123".to_string(),
                location: ApiKeyLocation::Header,
            }),
        };
        let curl = export_curl(&req, &[]);
        assert!(curl.contains("X-API-Key: abc123"));
    }

    #[test]
    fn test_export_curl_apikey_query() {
        let req = Request {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            method: Method::Get,
            url: "https://example.com".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: Some(Auth::ApiKey {
                key: "api_key".to_string(),
                value: "abc123".to_string(),
                location: ApiKeyLocation::Query,
            }),
        };
        let curl = export_curl(&req, &[]);
        assert!(curl.contains("api_key=abc123"));
    }

    #[test]
    fn test_export_filename_format() {
        let name = export_filename("Get Users", ExportFormat::PostmanV21);
        assert!(name.starts_with("get-users-"));
        assert!(name.ends_with(".postman_collection.json"));
    }

    #[test]
    fn test_export_filename_openapi() {
        let name = export_filename("My API", ExportFormat::OpenApi3);
        assert!(name.starts_with("my-api-"));
        assert!(name.ends_with(".openapi.json"));
    }

    #[test]
    fn test_exports_dir() {
        let dir = exports_dir();
        assert!(dir.ends_with("exports"));
    }

    #[test]
    fn test_format_labels() {
        assert_eq!(ExportFormat::Curl.label(), "cURL command");
        assert_eq!(ExportFormat::PostmanV21.label(), "Postman Collection v2.1");
        assert_eq!(ExportFormat::OpenApi3.label(), "OpenAPI 3.0");
    }

    #[test]
    fn test_request_formats_includes_curl() {
        let formats = ExportFormat::request_formats();
        assert!(formats.contains(&ExportFormat::Curl));
        assert_eq!(formats.len(), 3);
    }

    #[test]
    fn test_collection_formats_excludes_curl() {
        let formats = ExportFormat::collection_formats();
        assert!(!formats.contains(&ExportFormat::Curl));
        assert_eq!(formats.len(), 2);
    }
}
