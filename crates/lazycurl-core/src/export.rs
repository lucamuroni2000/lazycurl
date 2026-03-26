use crate::collection::slugify;
use crate::command::CurlCommandBuilder;
use crate::config::config_dir;
use crate::types::{ApiKeyLocation, Auth, Body, Collection, Request};

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

/// Export a single request as a Postman Collection v2.1 JSON object.
pub fn export_postman_request(request: &Request) -> serde_json::Value {
    let item = postman_item(request);
    serde_json::json!({
        "info": {
            "name": request.name,
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "item": [item],
        "variable": []
    })
}

/// Export a full collection as a Postman Collection v2.1 JSON object.
pub fn export_postman_collection(collection: &Collection) -> serde_json::Value {
    let items: Vec<serde_json::Value> = collection.requests.iter().map(postman_item).collect();
    let variables: Vec<serde_json::Value> = collection
        .variables
        .iter()
        .map(|(key, var)| {
            serde_json::json!({
                "key": key,
                "value": var.value,
                "type": "string"
            })
        })
        .collect();

    serde_json::json!({
        "info": {
            "name": collection.name,
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
        },
        "item": items,
        "variable": variables
    })
}

fn postman_item(request: &Request) -> serde_json::Value {
    let mut req_obj = serde_json::json!({
        "method": request.method.to_string(),
        "url": postman_url(request),
        "header": postman_headers(request),
    });

    if let Some(body) = postman_body(request) {
        req_obj["body"] = body;
    }

    if let Some(auth) = postman_auth(request) {
        req_obj["auth"] = auth;
    }

    serde_json::json!({
        "name": request.name,
        "request": req_obj
    })
}

fn postman_url(request: &Request) -> serde_json::Value {
    let query: Vec<serde_json::Value> = request
        .params
        .iter()
        .filter(|p| p.enabled)
        .map(|p| serde_json::json!({"key": p.key, "value": p.value}))
        .collect();

    serde_json::json!({
        "raw": request.url,
        "query": query
    })
}

fn postman_headers(request: &Request) -> Vec<serde_json::Value> {
    request
        .headers
        .iter()
        .filter(|h| h.enabled)
        .map(|h| serde_json::json!({"key": h.key, "value": h.value}))
        .collect()
}

fn postman_body(request: &Request) -> Option<serde_json::Value> {
    match &request.body {
        Some(Body::Json { content }) => Some(serde_json::json!({
            "mode": "raw",
            "raw": content,
            "options": { "raw": { "language": "json" } }
        })),
        Some(Body::Text { content }) => Some(serde_json::json!({
            "mode": "raw",
            "raw": content,
            "options": { "raw": { "language": "text" } }
        })),
        Some(Body::Form { fields }) => {
            let items: Vec<serde_json::Value> = fields
                .iter()
                .filter(|f| f.enabled)
                .map(|f| serde_json::json!({"key": f.key, "value": f.value}))
                .collect();
            Some(serde_json::json!({ "mode": "urlencoded", "urlencoded": items }))
        }
        Some(Body::Multipart { parts }) => {
            let items: Vec<serde_json::Value> = parts
                .iter()
                .map(|p| {
                    if let Some(path) = &p.file_path {
                        serde_json::json!({"key": p.name, "type": "file", "src": path})
                    } else {
                        serde_json::json!({"key": p.name, "value": p.value.as_deref().unwrap_or(""), "type": "text"})
                    }
                })
                .collect();
            Some(serde_json::json!({ "mode": "formdata", "formdata": items }))
        }
        Some(Body::None) | None => None,
    }
}

fn postman_auth(request: &Request) -> Option<serde_json::Value> {
    match &request.auth {
        Some(Auth::Bearer { token }) => Some(serde_json::json!({
            "type": "bearer",
            "bearer": [{"key": "token", "value": token, "type": "string"}]
        })),
        Some(Auth::Basic { username, password }) => Some(serde_json::json!({
            "type": "basic",
            "basic": [
                {"key": "username", "value": username, "type": "string"},
                {"key": "password", "value": password, "type": "string"}
            ]
        })),
        Some(Auth::ApiKey { key, value, location }) => {
            let loc = match location {
                ApiKeyLocation::Header => "header",
                ApiKeyLocation::Query => "query",
            };
            Some(serde_json::json!({
                "type": "apikey",
                "apikey": [
                    {"key": "key", "value": key, "type": "string"},
                    {"key": "value", "value": value, "type": "string"},
                    {"key": "in", "value": loc, "type": "string"}
                ]
            }))
        }
        Some(Auth::None) | None => None,
    }
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
    use crate::types::{Collection, FormField, Header, Method, Param, Variable};
    use std::collections::HashMap;

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

    #[test]
    fn test_export_postman_single_request() {
        let req = make_request();
        let json = export_postman_request(&req);
        let info = json["info"].as_object().unwrap();
        assert_eq!(info["name"].as_str().unwrap(), "Get Users");
        assert!(info["schema"].as_str().unwrap().contains("v2.1.0"));
        let items = json["item"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        let item_req = &items[0]["request"];
        assert_eq!(item_req["method"].as_str().unwrap(), "GET");
        let headers = item_req["header"].as_array().unwrap();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0]["key"].as_str().unwrap(), "Accept");
        let query = item_req["url"]["query"].as_array().unwrap();
        assert_eq!(query.len(), 1);
        assert_eq!(query[0]["key"].as_str().unwrap(), "page");
        let auth = &item_req["auth"];
        assert_eq!(auth["type"].as_str().unwrap(), "bearer");
    }

    #[test]
    fn test_export_postman_collection() {
        let collection = Collection {
            id: uuid::Uuid::new_v4(),
            name: "My API".to_string(),
            variables: {
                let mut m = HashMap::new();
                m.insert("base_url".to_string(), Variable { value: "https://api.example.com".to_string(), secret: false });
                m
            },
            requests: vec![make_request()],
        };
        let json = export_postman_collection(&collection);
        assert_eq!(json["info"]["name"].as_str().unwrap(), "My API");
        let items = json["item"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        let vars = json["variable"].as_array().unwrap();
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0]["key"].as_str().unwrap(), "base_url");
    }

    #[test]
    fn test_export_postman_json_body() {
        let req = Request {
            id: uuid::Uuid::new_v4(),
            name: "Create".to_string(),
            method: Method::Post,
            url: "https://example.com/api".to_string(),
            headers: vec![], params: vec![],
            body: Some(Body::Json { content: r#"{"key":"value"}"#.to_string() }),
            auth: None,
        };
        let json = export_postman_request(&req);
        let body = &json["item"][0]["request"]["body"];
        assert_eq!(body["mode"].as_str().unwrap(), "raw");
        assert_eq!(body["raw"].as_str().unwrap(), r#"{"key":"value"}"#);
        assert_eq!(body["options"]["raw"]["language"].as_str().unwrap(), "json");
    }

    #[test]
    fn test_export_postman_form_body() {
        let req = Request {
            id: uuid::Uuid::new_v4(),
            name: "Login".to_string(),
            method: Method::Post,
            url: "https://example.com/login".to_string(),
            headers: vec![], params: vec![],
            body: Some(Body::Form {
                fields: vec![
                    FormField { key: "user".to_string(), value: "alice".to_string(), enabled: true },
                ],
            }),
            auth: None,
        };
        let json = export_postman_request(&req);
        let body = &json["item"][0]["request"]["body"];
        assert_eq!(body["mode"].as_str().unwrap(), "urlencoded");
        let fields = body["urlencoded"].as_array().unwrap();
        assert_eq!(fields[0]["key"].as_str().unwrap(), "user");
    }

    #[test]
    fn test_export_postman_basic_auth() {
        let req = Request {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            method: Method::Get,
            url: "https://example.com".to_string(),
            headers: vec![], params: vec![], body: None,
            auth: Some(Auth::Basic { username: "user".to_string(), password: "pass".to_string() }),
        };
        let json = export_postman_request(&req);
        let auth = &json["item"][0]["request"]["auth"];
        assert_eq!(auth["type"].as_str().unwrap(), "basic");
        let basic = auth["basic"].as_array().unwrap();
        assert!(basic.iter().any(|v| v["key"] == "username" && v["value"] == "user"));
        assert!(basic.iter().any(|v| v["key"] == "password" && v["value"] == "pass"));
    }
}
