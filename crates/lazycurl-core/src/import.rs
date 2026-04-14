use std::fmt;
use std::path::Path;

use crate::types::{
    Auth, Body, Collection, FormField, Header, Method, MultipartPart, Param, RawBodyType, Request,
    Variable,
};

/// Supported import formats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportFormat {
    Curl,
    Postman,
    OpenAPI,
}

impl ImportFormat {
    pub fn all() -> &'static [ImportFormat] {
        &[
            ImportFormat::Curl,
            ImportFormat::Postman,
            ImportFormat::OpenAPI,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ImportFormat::Curl => "Curl command",
            ImportFormat::Postman => "Postman Collection",
            ImportFormat::OpenAPI => "OpenAPI Specification",
        }
    }
}

impl fmt::Display for ImportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Result of a successful import operation.
#[derive(Debug, Clone)]
pub struct ImportResult {
    pub collection: Collection,
    pub warnings: Vec<ImportWarning>,
}

/// A warning about something that couldn't be fully mapped during import.
#[derive(Debug, Clone)]
pub struct ImportWarning {
    pub request_name: Option<String>,
    pub message: String,
}

impl fmt::Display for ImportWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(name) = &self.request_name {
            write!(f, "\"{}\": {}", name, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

/// Errors that cause an import to fail entirely.
#[derive(Debug)]
pub enum ImportError {
    Io(std::io::Error),
    ParseError(String),
    UnsupportedFormat(String),
    InvalidCurl(String),
    EmptyImport,
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::Io(e) => write!(f, "I/O error: {}", e),
            ImportError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ImportError::UnsupportedFormat(msg) => write!(f, "Unsupported format: {}", msg),
            ImportError::InvalidCurl(msg) => write!(f, "Invalid curl command: {}", msg),
            ImportError::EmptyImport => write!(f, "No importable requests found"),
        }
    }
}

impl From<std::io::Error> for ImportError {
    fn from(e: std::io::Error) -> Self {
        ImportError::Io(e)
    }
}

impl From<serde_json::Error> for ImportError {
    fn from(e: serde_json::Error) -> Self {
        ImportError::ParseError(e.to_string())
    }
}

/// Parse a curl command string and return an ImportResult with one request.
pub fn import_curl(input: &str) -> Result<ImportResult, ImportError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ImportError::InvalidCurl("empty input".to_string()));
    }

    // Normalize multiline continuations
    let normalized = trimmed.replace("\\\r\n", " ").replace("\\\n", " ");

    // Strip leading "curl " or "curl.exe "
    let rest = if let Some(s) = normalized.strip_prefix("curl.exe ") {
        s
    } else if let Some(s) = normalized.strip_prefix("curl ") {
        s
    } else {
        &normalized
    };

    let tokens = shell_words::split(rest)
        .map_err(|e| ImportError::InvalidCurl(format!("shell parse error: {}", e)))?;

    let mut method: Option<Method> = None;
    let mut url: Option<String> = None;
    let mut headers: Vec<Header> = Vec::new();
    let mut raw_data: Option<String> = None;
    let mut form_parts: Vec<MultipartPart> = Vec::new();
    let mut form_fields: Vec<FormField> = Vec::new();
    let mut auth: Option<Auth> = None;
    let mut json_flag = false;
    let mut is_form = false;
    let mut is_multipart = false;

    let mut i = 0;
    while i < tokens.len() {
        let tok = &tokens[i];
        match tok.as_str() {
            "-X" | "--request" => {
                i += 1;
                if let Some(m) = tokens.get(i) {
                    method = Some(parse_method(m));
                }
            }
            "-H" | "--header" => {
                i += 1;
                if let Some(h) = tokens.get(i) {
                    if let Some(colon) = h.find(':') {
                        let key = h[..colon].trim().to_string();
                        let value = h[colon + 1..].trim().to_string();
                        headers.push(Header {
                            key,
                            value,
                            enabled: true,
                        });
                    }
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                i += 1;
                if let Some(d) = tokens.get(i) {
                    raw_data = Some(d.clone());
                }
            }
            "--json" => {
                i += 1;
                if let Some(d) = tokens.get(i) {
                    raw_data = Some(d.clone());
                    json_flag = true;
                }
            }
            "--data-urlencode" => {
                i += 1;
                if let Some(kv) = tokens.get(i) {
                    if let Some(eq) = kv.find('=') {
                        let key = kv[..eq].to_string();
                        let value = kv[eq + 1..].to_string();
                        form_fields.push(FormField {
                            key,
                            value,
                            enabled: true,
                        });
                        is_form = true;
                    } else {
                        form_fields.push(FormField {
                            key: kv.clone(),
                            value: String::new(),
                            enabled: true,
                        });
                        is_form = true;
                    }
                }
            }
            "-F" | "--form" => {
                i += 1;
                if let Some(kv) = tokens.get(i) {
                    if let Some(eq) = kv.find('=') {
                        let name = kv[..eq].to_string();
                        let val = &kv[eq + 1..];
                        if let Some(file) = val.strip_prefix('@') {
                            form_parts.push(MultipartPart {
                                name,
                                value: None,
                                file_path: Some(file.to_string()),
                            });
                        } else {
                            form_parts.push(MultipartPart {
                                name,
                                value: Some(val.to_string()),
                                file_path: None,
                            });
                        }
                        is_multipart = true;
                    }
                }
            }
            "-u" | "--user" => {
                i += 1;
                if let Some(creds) = tokens.get(i) {
                    if let Some(colon) = creds.find(':') {
                        auth = Some(Auth::Basic {
                            username: creds[..colon].to_string(),
                            password: creds[colon + 1..].to_string(),
                        });
                    } else {
                        auth = Some(Auth::Basic {
                            username: creds.clone(),
                            password: String::new(),
                        });
                    }
                }
            }
            "-b" | "--cookie" => {
                i += 1;
                if let Some(cookie) = tokens.get(i) {
                    headers.push(Header {
                        key: "Cookie".to_string(),
                        value: cookie.clone(),
                        enabled: true,
                    });
                }
            }
            // Silently skip irrelevant flags that consume a value
            "-o" | "--output" | "--proxy" | "-x" | "--connect-timeout" | "--max-time" | "-m"
            | "--cert" | "--key" | "--cacert" | "-w" | "--write-out" | "--retry" | "-e"
            | "--referer" => {
                i += 1; // skip the next value token
            }
            // Silently skip irrelevant boolean flags
            "-k" | "--insecure" | "-v" | "--verbose" | "-s" | "--silent" | "-S"
            | "--show-error" | "-L" | "--location" | "-i" | "--include" | "--compressed" => {}
            _ => {
                // First non-flag token that looks like a URL
                if url.is_none()
                    && !tok.starts_with('-')
                    && (tok.starts_with("http://")
                        || tok.starts_with("https://")
                        || tok.starts_with("{{"))
                {
                    url = Some(tok.clone());
                }
                // else: unknown flag or extra token — silently ignore
            }
        }
        i += 1;
    }

    let raw_url = url.ok_or_else(|| ImportError::InvalidCurl("no URL found".to_string()))?;
    let (base_url, params) = extract_query_params(&raw_url);

    // Handle Authorization: Bearer header → convert to Auth
    let mut final_headers: Vec<Header> = Vec::new();
    for h in headers {
        if h.key.eq_ignore_ascii_case("Authorization") {
            if let Some(token) = h.value.strip_prefix("Bearer ") {
                auth = Some(Auth::Bearer {
                    token: token.to_string(),
                });
                continue; // remove from headers
            }
        }
        final_headers.push(h);
    }

    // --json flag: add Content-Type and Accept headers, default to POST
    if json_flag {
        if method.is_none() {
            method = Some(Method::Post);
        }
        // Only add if not already present
        if !final_headers
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case("Content-Type"))
        {
            final_headers.push(Header {
                key: "Content-Type".to_string(),
                value: "application/json".to_string(),
                enabled: true,
            });
        }
        if !final_headers
            .iter()
            .any(|h| h.key.eq_ignore_ascii_case("Accept"))
        {
            final_headers.push(Header {
                key: "Accept".to_string(),
                value: "application/json".to_string(),
                enabled: true,
            });
        }
    }

    // Determine body
    let body = if is_multipart {
        Some(Body::Multipart { parts: form_parts })
    } else if is_form {
        Some(Body::Form {
            fields: form_fields,
        })
    } else if let Some(data) = raw_data {
        // Auto-detect JSON vs plain text
        let trimmed_data = data.trim();
        let is_json = trimmed_data.starts_with('{') || trimmed_data.starts_with('[');
        let content_type = if json_flag || is_json {
            RawBodyType::Json
        } else {
            RawBodyType::Text
        };
        // Infer POST if no explicit method
        if method.is_none() {
            method = Some(Method::Post);
        }
        Some(Body::Raw {
            content: data,
            content_type,
        })
    } else {
        None
    };

    let final_method = method.unwrap_or(Method::Get);

    let request = Request {
        id: uuid::Uuid::new_v4(),
        name: "Imported Request".to_string(),
        url: base_url,
        method: final_method,
        headers: final_headers,
        params,
        body,
        auth,
    };

    let collection = Collection {
        id: uuid::Uuid::new_v4(),
        name: "Imported Curl Request".to_string(),
        variables: std::collections::HashMap::new(),
        requests: vec![request],
    };

    Ok(ImportResult {
        collection,
        warnings: Vec::new(),
    })
}

/// Convert a method string to a Method enum value.
fn parse_method(s: &str) -> Method {
    match s.to_uppercase().as_str() {
        "POST" => Method::Post,
        "PUT" => Method::Put,
        "DELETE" => Method::Delete,
        "PATCH" => Method::Patch,
        "HEAD" => Method::Head,
        "OPTIONS" => Method::Options,
        _ => Method::Get,
    }
}

/// Split a URL into base URL and query params.
fn extract_query_params(url: &str) -> (String, Vec<Param>) {
    if let Some(q_pos) = url.find('?') {
        let base = url[..q_pos].to_string();
        let query = &url[q_pos + 1..];
        let params = query
            .split('&')
            .filter(|s| !s.is_empty())
            .map(|pair| {
                if let Some(eq) = pair.find('=') {
                    Param {
                        key: pair[..eq].to_string(),
                        value: pair[eq + 1..].to_string(),
                        enabled: true,
                    }
                } else {
                    Param {
                        key: pair.to_string(),
                        value: String::new(),
                        enabled: true,
                    }
                }
            })
            .collect();
        (base, params)
    } else {
        (url.to_string(), Vec::new())
    }
}

/// Import a Postman Collection v2.1 JSON file.
pub fn import_postman(path: &Path) -> Result<ImportResult, ImportError> {
    let content = std::fs::read_to_string(path)?;
    let root: serde_json::Value = serde_json::from_str(&content)?;

    // Validate schema
    let schema = root
        .get("info")
        .and_then(|i| i.get("schema"))
        .and_then(|s| s.as_str())
        .unwrap_or("");
    if !schema.contains("postman") {
        return Err(ImportError::UnsupportedFormat(
            "Not a Postman collection (missing schema field)".into(),
        ));
    }
    if !schema.contains("v2.1") && !schema.contains("v2.0") {
        return Err(ImportError::UnsupportedFormat(format!(
            "Unsupported Postman schema version: {}",
            schema
        )));
    }

    let name = root
        .get("info")
        .and_then(|i| i.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("Imported Collection")
        .to_string();

    // Parse collection variables
    let mut variables = std::collections::HashMap::new();
    if let Some(vars) = root.get("variable").and_then(|v| v.as_array()) {
        for var in vars {
            if let (Some(key), Some(value)) = (
                var.get("key").and_then(|k| k.as_str()),
                var.get("value").and_then(|v| v.as_str()),
            ) {
                variables.insert(
                    key.to_string(),
                    Variable {
                        value: value.to_string(),
                        secret: false,
                    },
                );
            }
        }
    }

    // Flatten and parse items
    let mut requests = Vec::new();
    let mut warnings = Vec::new();
    if let Some(items) = root.get("item").and_then(|i| i.as_array()) {
        flatten_postman_items(items, &mut requests, &mut warnings);
    }

    if requests.is_empty() {
        return Err(ImportError::EmptyImport);
    }

    let collection = Collection {
        id: uuid::Uuid::new_v4(),
        name,
        variables,
        requests,
    };

    Ok(ImportResult {
        collection,
        warnings,
    })
}

fn flatten_postman_items(
    items: &[serde_json::Value],
    requests: &mut Vec<Request>,
    warnings: &mut Vec<ImportWarning>,
) {
    for item in items {
        // If item has nested "item" array, it's a folder — recurse
        if let Some(sub_items) = item.get("item").and_then(|i| i.as_array()) {
            flatten_postman_items(sub_items, requests, warnings);
            continue;
        }

        // Otherwise it's a request
        let request_obj = match item.get("request") {
            Some(r) => r,
            None => continue,
        };

        let item_name = item
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("Unnamed Request")
            .to_string();

        match parse_postman_request(request_obj, &item_name) {
            Ok(req) => requests.push(req),
            Err(msg) => warnings.push(ImportWarning {
                request_name: Some(item_name),
                message: msg,
            }),
        }
    }
}

fn parse_postman_request(req: &serde_json::Value, name: &str) -> Result<Request, String> {
    let method_str = req.get("method").and_then(|m| m.as_str()).unwrap_or("GET");
    let method = parse_method(method_str);

    // URL can be a string or an object with "raw"
    let (url, params) = parse_postman_url(req.get("url"));

    // Headers
    let headers = parse_postman_headers(req.get("header"));

    // Body
    let body = parse_postman_body(req.get("body"));

    // Auth
    let auth = parse_postman_auth(req.get("auth"));

    Ok(Request {
        id: uuid::Uuid::new_v4(),
        name: name.to_string(),
        method,
        url,
        headers,
        params,
        body,
        auth,
    })
}

fn parse_postman_url(url_val: Option<&serde_json::Value>) -> (String, Vec<Param>) {
    let url_val = match url_val {
        Some(v) => v,
        None => return (String::new(), Vec::new()),
    };

    // URL can be a plain string
    if let Some(s) = url_val.as_str() {
        return extract_query_params(s);
    }

    let raw = url_val
        .get("raw")
        .and_then(|r| r.as_str())
        .unwrap_or("")
        .to_string();

    // If structured query params exist, use those (they have disabled state)
    if let Some(query) = url_val.get("query").and_then(|q| q.as_array()) {
        let base = raw.split('?').next().unwrap_or(&raw).to_string();
        let params = query
            .iter()
            .filter_map(|p| {
                let key = p.get("key").and_then(|k| k.as_str())?.to_string();
                let value = p
                    .get("value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let enabled = !p.get("disabled").and_then(|d| d.as_bool()).unwrap_or(false);
                Some(Param {
                    key,
                    value,
                    enabled,
                })
            })
            .collect();
        return (base, params);
    }

    // Fallback: extract from raw URL
    extract_query_params(&raw)
}

fn parse_postman_headers(header_val: Option<&serde_json::Value>) -> Vec<Header> {
    let arr = match header_val.and_then(|h| h.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };

    arr.iter()
        .filter_map(|h| {
            let key = h.get("key").and_then(|k| k.as_str())?.to_string();
            let value = h
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let enabled = !h.get("disabled").and_then(|d| d.as_bool()).unwrap_or(false);
            Some(Header {
                key,
                value,
                enabled,
            })
        })
        .collect()
}

fn parse_postman_body(body_val: Option<&serde_json::Value>) -> Option<Body> {
    let body = body_val?;
    let mode = body.get("mode").and_then(|m| m.as_str()).unwrap_or("");

    match mode {
        "raw" => {
            let content = body
                .get("raw")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .to_string();
            let language = body
                .get("options")
                .and_then(|o| o.get("raw"))
                .and_then(|r| r.get("language"))
                .and_then(|l| l.as_str())
                .unwrap_or("");
            let content_type = match language {
                "json" => RawBodyType::Json,
                "xml" => RawBodyType::Xml,
                "html" => RawBodyType::Html,
                "javascript" => RawBodyType::Javascript,
                _ => {
                    if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                        RawBodyType::Json
                    } else {
                        RawBodyType::Text
                    }
                }
            };
            Some(Body::Raw {
                content,
                content_type,
            })
        }
        "formdata" => {
            let parts = body
                .get("formdata")
                .and_then(|f| f.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|p| {
                            let name = p.get("key").and_then(|k| k.as_str())?.to_string();
                            let field_type =
                                p.get("type").and_then(|t| t.as_str()).unwrap_or("text");
                            if field_type == "file" {
                                let file_path = p
                                    .get("src")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                Some(MultipartPart {
                                    name,
                                    value: None,
                                    file_path: Some(file_path),
                                })
                            } else {
                                let value = p
                                    .get("value")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                Some(MultipartPart {
                                    name,
                                    value: Some(value),
                                    file_path: None,
                                })
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(Body::Multipart { parts })
        }
        "urlencoded" => {
            let fields = body
                .get("urlencoded")
                .and_then(|u| u.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|f| {
                            let key = f.get("key").and_then(|k| k.as_str())?.to_string();
                            let value = f
                                .get("value")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let enabled =
                                !f.get("disabled").and_then(|d| d.as_bool()).unwrap_or(false);
                            Some(FormField {
                                key,
                                value,
                                enabled,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(Body::Form { fields })
        }
        "graphql" => {
            let gql = body.get("graphql")?;
            let query = gql
                .get("query")
                .and_then(|q| q.as_str())
                .unwrap_or("")
                .to_string();
            let variables = gql
                .get("variables")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some(Body::GraphQL { query, variables })
        }
        "file" => {
            let file_path = body
                .get("file")
                .and_then(|f| f.get("src"))
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            Some(Body::Binary { file_path })
        }
        _ => None,
    }
}

fn parse_postman_auth(auth_val: Option<&serde_json::Value>) -> Option<Auth> {
    let auth = auth_val?;
    let auth_type = auth.get("type").and_then(|t| t.as_str())?;

    match auth_type {
        "bearer" => {
            let token = get_postman_auth_field(auth, "bearer", "token")?;
            Some(Auth::Bearer { token })
        }
        "basic" => {
            let username = get_postman_auth_field(auth, "basic", "username").unwrap_or_default();
            let password = get_postman_auth_field(auth, "basic", "password").unwrap_or_default();
            Some(Auth::Basic { username, password })
        }
        "apikey" => {
            let key = get_postman_auth_field(auth, "apikey", "key").unwrap_or_default();
            let value = get_postman_auth_field(auth, "apikey", "value").unwrap_or_default();
            let location_str = get_postman_auth_field(auth, "apikey", "in")
                .unwrap_or_else(|| "header".to_string());
            let location = match location_str.as_str() {
                "query" => crate::types::ApiKeyLocation::Query,
                _ => crate::types::ApiKeyLocation::Header,
            };
            Some(Auth::ApiKey {
                key,
                value,
                location,
            })
        }
        "noauth" => Some(Auth::None),
        _ => None,
    }
}

/// Extract a field value from Postman's auth array format:
/// `{ "type": "bearer", "bearer": [{"key": "token", "value": "xxx"}] }`
fn get_postman_auth_field(
    auth: &serde_json::Value,
    auth_type: &str,
    field_key: &str,
) -> Option<String> {
    auth.get(auth_type)
        .and_then(|arr| arr.as_array())
        .and_then(|arr| {
            arr.iter().find_map(|entry| {
                if entry.get("key").and_then(|k| k.as_str()) == Some(field_key) {
                    entry
                        .get("value")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                } else {
                    None
                }
            })
        })
}

/// Import an OpenAPI 3.x specification file (JSON or YAML).
pub fn import_openapi(path: &Path) -> Result<ImportResult, ImportError> {
    let content = std::fs::read_to_string(path)?;

    // Try JSON first, then YAML
    let root: serde_json::Value = serde_json::from_str(&content).or_else(|_| {
        serde_yaml::from_str(&content)
            .map_err(|e| ImportError::ParseError(format!("JSON/YAML parse error: {}", e)))
    })?;

    // Reject Swagger 2.0
    if root.get("swagger").is_some() {
        return Err(ImportError::UnsupportedFormat(
            "Swagger 2.0 is not supported. Please convert to OpenAPI 3.x.".into(),
        ));
    }

    // Validate OpenAPI 3.x
    if root.get("openapi").is_none() {
        return Err(ImportError::UnsupportedFormat(
            "Not an OpenAPI specification (missing 'openapi' field)".into(),
        ));
    }

    let title = root
        .get("info")
        .and_then(|i| i.get("title"))
        .and_then(|t| t.as_str())
        .unwrap_or("Imported OpenAPI")
        .to_string();

    let base_url = root
        .get("servers")
        .and_then(|s| s.as_array())
        .and_then(|arr| arr.first())
        .and_then(|s| s.get("url"))
        .and_then(|u| u.as_str())
        .unwrap_or("")
        .trim_end_matches('/');

    // Collect security schemes for auth mapping
    let security_schemes = root
        .get("components")
        .and_then(|c| c.get("securitySchemes"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let paths = root
        .get("paths")
        .and_then(|p| p.as_object())
        .ok_or(ImportError::ParseError("Missing 'paths' object".into()))?;

    let mut requests = Vec::new();
    let warnings = Vec::new();

    let http_methods = ["get", "post", "put", "delete", "patch", "head", "options"];

    for (path_str, path_item) in paths {
        let path_obj = match path_item.as_object() {
            Some(o) => o,
            None => continue,
        };

        // Path-level parameters
        let path_params = path_obj
            .get("parameters")
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default();

        for method_str in &http_methods {
            let operation = match path_obj.get(*method_str) {
                Some(op) => op,
                None => continue,
            };

            let name = operation
                .get("operationId")
                .and_then(|o| o.as_str())
                .map(String::from)
                .unwrap_or_else(|| format!("{} {}", method_str.to_uppercase(), path_str));

            let method = parse_method(method_str);
            let url = if base_url.is_empty() {
                path_str.to_string()
            } else {
                format!("{}{}", base_url, path_str)
            };

            // Merge path-level and operation-level parameters
            let op_params = operation
                .get("parameters")
                .and_then(|p| p.as_array())
                .cloned()
                .unwrap_or_default();

            let all_params: Vec<serde_json::Value> = path_params
                .iter()
                .chain(op_params.iter())
                .cloned()
                .collect();

            let mut headers = Vec::new();
            let mut params = Vec::new();

            for param in &all_params {
                let param_name = match param.get("name").and_then(|n| n.as_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                let param_in = param.get("in").and_then(|i| i.as_str()).unwrap_or("");

                // Extract example value: param.example > param.schema.example
                let example_val = param
                    .get("example")
                    .or_else(|| param.get("schema").and_then(|s| s.get("example")))
                    .map(|v| match v.as_str() {
                        Some(s) => s.to_string(),
                        None => v.to_string(),
                    })
                    .unwrap_or_default();

                match param_in {
                    "query" => params.push(Param {
                        key: param_name,
                        value: example_val,
                        enabled: true,
                    }),
                    "header" => headers.push(Header {
                        key: param_name,
                        value: example_val,
                        enabled: true,
                    }),
                    _ => {} // path and cookie params: not directly mapped
                }
            }

            // Request body
            let body = parse_openapi_body(operation.get("requestBody"));

            // Auth from security
            let auth = parse_openapi_security(
                operation.get("security"),
                root.get("security"),
                &security_schemes,
            );

            requests.push(Request {
                id: uuid::Uuid::new_v4(),
                name,
                method,
                url,
                headers,
                params,
                body,
                auth,
            });
        }
    }

    if requests.is_empty() {
        return Err(ImportError::EmptyImport);
    }

    let collection = Collection {
        id: uuid::Uuid::new_v4(),
        name: title,
        variables: std::collections::HashMap::new(),
        requests,
    };

    Ok(ImportResult {
        collection,
        warnings,
    })
}

fn parse_openapi_body(body_val: Option<&serde_json::Value>) -> Option<Body> {
    let body = body_val?;
    let content = body.get("content")?.as_object()?;

    // Prefer application/json, then first available
    let (content_type_str, media) = if let Some(json_media) = content.get("application/json") {
        ("application/json", json_media)
    } else if let Some((ct, media)) = content.iter().next() {
        (ct.as_str(), media)
    } else {
        return None;
    };

    // Try example first, then generate from schema
    let example = media
        .get("example")
        .or_else(|| media.get("schema").and_then(|s| s.get("example")));

    if content_type_str.contains("json") {
        let content_str = match example {
            Some(ex) => serde_json::to_string_pretty(ex).unwrap_or_default(),
            None => String::new(),
        };
        Some(Body::Raw {
            content: content_str,
            content_type: RawBodyType::Json,
        })
    } else if content_type_str.contains("xml") {
        Some(Body::Raw {
            content: example.and_then(|e| e.as_str()).unwrap_or("").to_string(),
            content_type: RawBodyType::Xml,
        })
    } else {
        Some(Body::Raw {
            content: example.and_then(|e| e.as_str()).unwrap_or("").to_string(),
            content_type: RawBodyType::Text,
        })
    }
}

fn parse_openapi_security(
    operation_security: Option<&serde_json::Value>,
    global_security: Option<&serde_json::Value>,
    schemes: &serde_json::Value,
) -> Option<Auth> {
    // Operation-level security overrides global
    let security = operation_security.or(global_security)?;
    let arr = security.as_array()?;
    let first = arr.first()?.as_object()?;
    let (scheme_name, _) = first.iter().next()?;

    let scheme = schemes.get(scheme_name)?;
    let scheme_type = scheme.get("type").and_then(|t| t.as_str())?;

    match scheme_type {
        "http" => {
            let http_scheme = scheme.get("scheme").and_then(|s| s.as_str()).unwrap_or("");
            match http_scheme {
                "bearer" => Some(Auth::Bearer {
                    token: String::new(),
                }),
                "basic" => Some(Auth::Basic {
                    username: String::new(),
                    password: String::new(),
                }),
                _ => None,
            }
        }
        "apiKey" => {
            let name = scheme
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            let location_str = scheme
                .get("in")
                .and_then(|i| i.as_str())
                .unwrap_or("header");
            let location = match location_str {
                "query" => crate::types::ApiKeyLocation::Query,
                _ => crate::types::ApiKeyLocation::Header,
            };
            Some(Auth::ApiKey {
                key: name,
                value: String::new(),
                location,
            })
        }
        _ => None,
    }
}

/// Unified import entry point.
/// For Curl, `source` is the curl command string.
/// For Postman/OpenAPI, `source` is the file path.
pub fn import(format: &ImportFormat, source: &str) -> Result<ImportResult, ImportError> {
    match format {
        ImportFormat::Curl => import_curl(source),
        ImportFormat::Postman => import_postman(Path::new(source)),
        ImportFormat::OpenAPI => import_openapi(Path::new(source)),
    }
}

/// Detect import format from file content.
pub fn detect_format(content: &str) -> Result<ImportFormat, ImportError> {
    let trimmed = content.trim();
    if trimmed.starts_with("curl ") || trimmed.starts_with("curl.exe ") {
        return Ok(ImportFormat::Curl);
    }
    // Try JSON first
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if json
            .get("info")
            .and_then(|i| i.get("schema"))
            .and_then(|s| s.as_str())
            .map(|s| s.contains("postman"))
            .unwrap_or(false)
        {
            return Ok(ImportFormat::Postman);
        }
        if json.get("openapi").is_some() {
            return Ok(ImportFormat::OpenAPI);
        }
    }
    // Try YAML
    if let Ok(yaml) = serde_yaml::from_str::<serde_json::Value>(trimmed) {
        if yaml.get("openapi").is_some() {
            return Ok(ImportFormat::OpenAPI);
        }
    }
    Err(ImportError::UnsupportedFormat(
        "Could not detect format. Use --format to specify.".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_format_labels() {
        assert_eq!(ImportFormat::Curl.label(), "Curl command");
        assert_eq!(ImportFormat::Postman.label(), "Postman Collection");
        assert_eq!(ImportFormat::OpenAPI.label(), "OpenAPI Specification");
    }

    #[test]
    fn test_import_format_all() {
        let all = ImportFormat::all();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0], ImportFormat::Curl);
        assert_eq!(all[1], ImportFormat::Postman);
        assert_eq!(all[2], ImportFormat::OpenAPI);
    }

    #[test]
    fn test_import_warning_display_with_name() {
        let w = ImportWarning {
            request_name: Some("Get Users".into()),
            message: "unsupported auth type".into(),
        };
        assert_eq!(w.to_string(), "\"Get Users\": unsupported auth type");
    }

    #[test]
    fn test_import_warning_display_without_name() {
        let w = ImportWarning {
            request_name: None,
            message: "empty collection".into(),
        };
        assert_eq!(w.to_string(), "empty collection");
    }

    #[test]
    fn test_import_error_display() {
        assert_eq!(
            ImportError::EmptyImport.to_string(),
            "No importable requests found"
        );
        assert_eq!(
            ImportError::InvalidCurl("no URL".into()).to_string(),
            "Invalid curl command: no URL"
        );
    }

    #[test]
    fn test_detect_format_curl() {
        assert_eq!(
            detect_format("curl https://example.com").unwrap(),
            ImportFormat::Curl
        );
        assert_eq!(
            detect_format("  curl -X POST https://api.test").unwrap(),
            ImportFormat::Curl
        );
    }

    #[test]
    fn test_detect_format_curl_exe() {
        assert_eq!(
            detect_format("curl.exe https://example.com").unwrap(),
            ImportFormat::Curl
        );
    }

    #[test]
    fn test_detect_format_postman() {
        let json = r#"{"info":{"name":"Test","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},"item":[]}"#;
        assert_eq!(detect_format(json).unwrap(), ImportFormat::Postman);
    }

    #[test]
    fn test_detect_format_openapi_json() {
        let json = r#"{"openapi":"3.0.3","info":{"title":"Test","version":"1.0"},"paths":{}}"#;
        assert_eq!(detect_format(json).unwrap(), ImportFormat::OpenAPI);
    }

    #[test]
    fn test_detect_format_openapi_yaml() {
        let yaml = "openapi: '3.0.3'\ninfo:\n  title: Test\n  version: '1.0'\npaths: {}";
        assert_eq!(detect_format(yaml).unwrap(), ImportFormat::OpenAPI);
    }

    #[test]
    fn test_detect_format_unknown() {
        assert!(detect_format("hello world").is_err());
    }

    // ----- import_curl tests -----

    use crate::types::{Auth, Body, Method, RawBodyType};

    #[test]
    fn test_curl_basic_get() {
        let result = import_curl("curl https://example.com").unwrap();
        assert_eq!(result.collection.requests.len(), 1);
        let req = &result.collection.requests[0];
        assert_eq!(req.method, Method::Get);
        assert_eq!(req.url, "https://example.com");
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_curl_post_with_json() {
        let result = import_curl(r#"curl -X POST https://api.test/users -H "Content-Type: application/json" -d '{"name":"Alice"}'"#).unwrap();
        let req = &result.collection.requests[0];
        assert_eq!(req.method, Method::Post);
        assert_eq!(req.url, "https://api.test/users");
        assert_eq!(req.headers.len(), 1);
        assert_eq!(req.headers[0].key, "Content-Type");
        assert_eq!(req.headers[0].value, "application/json");
        match &req.body {
            Some(Body::Raw {
                content,
                content_type,
            }) => {
                assert_eq!(content, r#"{"name":"Alice"}"#);
                assert_eq!(*content_type, RawBodyType::Json);
            }
            other => panic!("Expected Raw JSON body, got {:?}", other),
        }
    }

    #[test]
    fn test_curl_json_flag() {
        let result = import_curl(r#"curl --json '{"key":"val"}' https://api.test"#).unwrap();
        let req = &result.collection.requests[0];
        assert_eq!(req.method, Method::Post);
        match &req.body {
            Some(Body::Raw {
                content,
                content_type,
            }) => {
                assert_eq!(content, r#"{"key":"val"}"#);
                assert_eq!(*content_type, RawBodyType::Json);
            }
            other => panic!("Expected Raw JSON body, got {:?}", other),
        }
        assert!(req
            .headers
            .iter()
            .any(|h| h.key == "Content-Type" && h.value == "application/json"));
        assert!(req
            .headers
            .iter()
            .any(|h| h.key == "Accept" && h.value == "application/json"));
    }

    #[test]
    fn test_curl_basic_auth() {
        let result = import_curl("curl -u alice:secret123 https://api.test").unwrap();
        let req = &result.collection.requests[0];
        match &req.auth {
            Some(Auth::Basic { username, password }) => {
                assert_eq!(username, "alice");
                assert_eq!(password, "secret123");
            }
            other => panic!("Expected Basic auth, got {:?}", other),
        }
    }

    #[test]
    fn test_curl_bearer_token() {
        let result =
            import_curl(r#"curl -H "Authorization: Bearer tok123" https://api.test"#).unwrap();
        let req = &result.collection.requests[0];
        match &req.auth {
            Some(Auth::Bearer { token }) => {
                assert_eq!(token, "tok123");
            }
            other => panic!("Expected Bearer auth, got {:?}", other),
        }
        assert!(!req.headers.iter().any(|h| h.key == "Authorization"));
    }

    #[test]
    fn test_curl_multiple_headers() {
        let result =
            import_curl(r#"curl -H "Accept: text/html" -H "X-Custom: foo" https://test.com"#)
                .unwrap();
        let req = &result.collection.requests[0];
        assert_eq!(req.headers.len(), 2);
        assert_eq!(req.headers[0].key, "Accept");
        assert_eq!(req.headers[1].key, "X-Custom");
    }

    #[test]
    fn test_curl_form_data() {
        let result =
            import_curl(r#"curl -X POST -F "name=Alice" -F "avatar=@photo.jpg" https://api.test"#)
                .unwrap();
        let req = &result.collection.requests[0];
        match &req.body {
            Some(Body::Multipart { parts }) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0].name, "name");
                assert_eq!(parts[0].value, Some("Alice".into()));
                assert!(parts[0].file_path.is_none());
                assert_eq!(parts[1].name, "avatar");
                assert!(parts[1].value.is_none());
                assert_eq!(parts[1].file_path, Some("photo.jpg".into()));
            }
            other => panic!("Expected Multipart body, got {:?}", other),
        }
    }

    #[test]
    fn test_curl_url_encoded_form() {
        let result =
            import_curl(r#"curl -X POST --data-urlencode "q=hello world" https://api.test"#)
                .unwrap();
        let req = &result.collection.requests[0];
        match &req.body {
            Some(Body::Form { fields }) => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].key, "q");
                assert_eq!(fields[0].value, "hello world");
            }
            other => panic!("Expected Form body, got {:?}", other),
        }
    }

    #[test]
    fn test_curl_query_params_from_url() {
        let result = import_curl("curl 'https://api.test/search?q=rust&page=1'").unwrap();
        let req = &result.collection.requests[0];
        assert_eq!(req.url, "https://api.test/search");
        assert_eq!(req.params.len(), 2);
        assert_eq!(req.params[0].key, "q");
        assert_eq!(req.params[0].value, "rust");
        assert_eq!(req.params[1].key, "page");
        assert_eq!(req.params[1].value, "1");
    }

    #[test]
    fn test_curl_multiline_backslash() {
        let input =
            "curl \\\n  -X POST \\\n  -H 'Content-Type: application/json' \\\n  https://api.test";
        let result = import_curl(input).unwrap();
        let req = &result.collection.requests[0];
        assert_eq!(req.method, Method::Post);
        assert_eq!(req.url, "https://api.test");
    }

    #[test]
    fn test_curl_cookie() {
        let result = import_curl(r#"curl -b "session=abc123" https://api.test"#).unwrap();
        let req = &result.collection.requests[0];
        assert!(req
            .headers
            .iter()
            .any(|h| h.key == "Cookie" && h.value == "session=abc123"));
    }

    #[test]
    fn test_curl_empty_input() {
        assert!(import_curl("").is_err());
    }

    #[test]
    fn test_curl_no_url() {
        assert!(import_curl("curl -X GET").is_err());
    }

    #[test]
    fn test_curl_data_auto_post() {
        let result = import_curl(r#"curl -d "key=val" https://api.test"#).unwrap();
        assert_eq!(result.collection.requests[0].method, Method::Post);
    }

    #[test]
    fn test_curl_collection_name() {
        let result = import_curl("curl https://example.com").unwrap();
        assert_eq!(result.collection.name, "Imported Curl Request");
    }

    // Helper to write JSON to a temp file
    fn write_temp_json(content: &str) -> tempfile::NamedTempFile {
        use std::io::Write;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_postman_minimal() {
        let json = r#"{
            "info": {
                "name": "My API",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Get Users",
                    "request": {
                        "method": "GET",
                        "url": {
                            "raw": "https://api.test/users",
                            "host": ["api", "test"],
                            "path": ["users"]
                        }
                    }
                }
            ]
        }"#;
        let path = write_temp_json(json);
        let result = import_postman(path.path()).unwrap();
        assert_eq!(result.collection.name, "My API");
        assert_eq!(result.collection.requests.len(), 1);
        assert_eq!(result.collection.requests[0].name, "Get Users");
        assert_eq!(result.collection.requests[0].method, Method::Get);
        assert_eq!(result.collection.requests[0].url, "https://api.test/users");
    }

    #[test]
    fn test_postman_with_variables() {
        let json = r#"{
            "info": {
                "name": "Var Test",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Test",
                    "request": {
                        "method": "GET",
                        "url": {
                            "raw": "{{base_url}}/users"
                        }
                    }
                }
            ],
            "variable": [
                {"key": "base_url", "value": "https://api.test"},
                {"key": "api_key", "value": "secret123"}
            ]
        }"#;
        let path = write_temp_json(json);
        let result = import_postman(path.path()).unwrap();
        assert_eq!(result.collection.variables.len(), 2);
        assert_eq!(
            result.collection.variables["base_url"].value,
            "https://api.test"
        );
        assert_eq!(result.collection.variables["api_key"].value, "secret123");
        assert_eq!(result.collection.requests[0].url, "{{base_url}}/users");
    }

    #[test]
    fn test_postman_nested_folders() {
        let json = r#"{
            "info": {
                "name": "Nested",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Users Folder",
                    "item": [
                        {
                            "name": "Get Users",
                            "request": {
                                "method": "GET",
                                "url": {"raw": "https://api.test/users"}
                            }
                        },
                        {
                            "name": "Inner Folder",
                            "item": [
                                {
                                    "name": "Deep Request",
                                    "request": {
                                        "method": "POST",
                                        "url": {"raw": "https://api.test/deep"}
                                    }
                                }
                            ]
                        }
                    ]
                }
            ]
        }"#;
        let path = write_temp_json(json);
        let result = import_postman(path.path()).unwrap();
        assert_eq!(result.collection.requests.len(), 2);
        assert_eq!(result.collection.requests[0].name, "Get Users");
        assert_eq!(result.collection.requests[1].name, "Deep Request");
    }

    #[test]
    fn test_postman_headers_and_body() {
        let json = r#"{
            "info": {
                "name": "Body Test",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Create User",
                    "request": {
                        "method": "POST",
                        "url": {"raw": "https://api.test/users"},
                        "header": [
                            {"key": "Content-Type", "value": "application/json", "disabled": false},
                            {"key": "X-Debug", "value": "true", "disabled": true}
                        ],
                        "body": {
                            "mode": "raw",
                            "raw": "{\"name\":\"Alice\"}",
                            "options": {"raw": {"language": "json"}}
                        }
                    }
                }
            ]
        }"#;
        let path = write_temp_json(json);
        let result = import_postman(path.path()).unwrap();
        let req = &result.collection.requests[0];
        assert_eq!(req.headers.len(), 2);
        assert!(req.headers[0].enabled);
        assert!(!req.headers[1].enabled);
        match &req.body {
            Some(Body::Raw {
                content,
                content_type,
            }) => {
                assert_eq!(content, r#"{"name":"Alice"}"#);
                assert_eq!(*content_type, RawBodyType::Json);
            }
            other => panic!("Expected Raw JSON body, got {:?}", other),
        }
    }

    #[test]
    fn test_postman_auth_bearer() {
        let json = r#"{
            "info": {
                "name": "Auth Test",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Authed",
                    "request": {
                        "method": "GET",
                        "url": {"raw": "https://api.test"},
                        "auth": {
                            "type": "bearer",
                            "bearer": [
                                {"key": "token", "value": "mytoken123"}
                            ]
                        }
                    }
                }
            ]
        }"#;
        let path = write_temp_json(json);
        let result = import_postman(path.path()).unwrap();
        match &result.collection.requests[0].auth {
            Some(Auth::Bearer { token }) => assert_eq!(token, "mytoken123"),
            other => panic!("Expected Bearer auth, got {:?}", other),
        }
    }

    #[test]
    fn test_postman_auth_basic() {
        let json = r#"{
            "info": {
                "name": "Basic Auth",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Basic",
                    "request": {
                        "method": "GET",
                        "url": {"raw": "https://api.test"},
                        "auth": {
                            "type": "basic",
                            "basic": [
                                {"key": "username", "value": "alice"},
                                {"key": "password", "value": "pass123"}
                            ]
                        }
                    }
                }
            ]
        }"#;
        let path = write_temp_json(json);
        let result = import_postman(path.path()).unwrap();
        match &result.collection.requests[0].auth {
            Some(Auth::Basic { username, password }) => {
                assert_eq!(username, "alice");
                assert_eq!(password, "pass123");
            }
            other => panic!("Expected Basic auth, got {:?}", other),
        }
    }

    #[test]
    fn test_postman_formdata() {
        let json = r#"{
            "info": {
                "name": "Form Test",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Form Post",
                    "request": {
                        "method": "POST",
                        "url": {"raw": "https://api.test/upload"},
                        "body": {
                            "mode": "formdata",
                            "formdata": [
                                {"key": "name", "value": "Alice", "type": "text"},
                                {"key": "file", "src": "/path/to/file.png", "type": "file"}
                            ]
                        }
                    }
                }
            ]
        }"#;
        let path = write_temp_json(json);
        let result = import_postman(path.path()).unwrap();
        match &result.collection.requests[0].body {
            Some(Body::Multipart { parts }) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0].name, "name");
                assert_eq!(parts[0].value, Some("Alice".into()));
                assert_eq!(parts[1].name, "file");
                assert_eq!(parts[1].file_path, Some("/path/to/file.png".into()));
            }
            other => panic!("Expected Multipart body, got {:?}", other),
        }
    }

    #[test]
    fn test_postman_urlencoded() {
        let json = r#"{
            "info": {
                "name": "Urlenc Test",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Login",
                    "request": {
                        "method": "POST",
                        "url": {"raw": "https://api.test/login"},
                        "body": {
                            "mode": "urlencoded",
                            "urlencoded": [
                                {"key": "user", "value": "alice", "disabled": false},
                                {"key": "pass", "value": "secret", "disabled": false}
                            ]
                        }
                    }
                }
            ]
        }"#;
        let path = write_temp_json(json);
        let result = import_postman(path.path()).unwrap();
        match &result.collection.requests[0].body {
            Some(Body::Form { fields }) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].key, "user");
                assert_eq!(fields[1].key, "pass");
            }
            other => panic!("Expected Form body, got {:?}", other),
        }
    }

    #[test]
    fn test_postman_graphql() {
        let json = r#"{
            "info": {
                "name": "GQL Test",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Query",
                    "request": {
                        "method": "POST",
                        "url": {"raw": "https://api.test/graphql"},
                        "body": {
                            "mode": "graphql",
                            "graphql": {
                                "query": "{ users { id name } }",
                                "variables": "{\"limit\": 10}"
                            }
                        }
                    }
                }
            ]
        }"#;
        let path = write_temp_json(json);
        let result = import_postman(path.path()).unwrap();
        match &result.collection.requests[0].body {
            Some(Body::GraphQL { query, variables }) => {
                assert_eq!(query, "{ users { id name } }");
                assert_eq!(variables, "{\"limit\": 10}");
            }
            other => panic!("Expected GraphQL body, got {:?}", other),
        }
    }

    #[test]
    fn test_postman_query_params() {
        let json = r#"{
            "info": {
                "name": "Params Test",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Search",
                    "request": {
                        "method": "GET",
                        "url": {
                            "raw": "https://api.test/search?q=rust&page=1",
                            "query": [
                                {"key": "q", "value": "rust"},
                                {"key": "page", "value": "1", "disabled": true}
                            ]
                        }
                    }
                }
            ]
        }"#;
        let path = write_temp_json(json);
        let result = import_postman(path.path()).unwrap();
        let req = &result.collection.requests[0];
        assert!(!req.url.contains('?'));
        assert_eq!(req.params.len(), 2);
        assert!(req.params[0].enabled);
        assert!(!req.params[1].enabled);
    }

    #[test]
    fn test_postman_invalid_json() {
        let path = write_temp_json("not json at all{{{");
        assert!(import_postman(path.path()).is_err());
    }

    #[test]
    fn test_postman_empty_items() {
        let json = r#"{
            "info": {
                "name": "Empty",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": []
        }"#;
        let path = write_temp_json(json);
        let result = import_postman(path.path());
        assert!(matches!(result, Err(ImportError::EmptyImport)));
    }

    #[test]
    fn test_postman_url_as_string() {
        let json = r#"{
            "info": {
                "name": "String URL",
                "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
            },
            "item": [
                {
                    "name": "Simple",
                    "request": {
                        "method": "GET",
                        "url": "https://api.test/simple"
                    }
                }
            ]
        }"#;
        let path = write_temp_json(json);
        let result = import_postman(path.path()).unwrap();
        assert_eq!(result.collection.requests[0].url, "https://api.test/simple");
    }

    #[test]
    fn test_openapi_minimal_json() {
        let json = r#"{
            "openapi": "3.0.3",
            "info": {"title": "Pet API", "version": "1.0"},
            "servers": [{"url": "https://api.test"}],
            "paths": {
                "/pets": {
                    "get": {
                        "operationId": "listPets",
                        "summary": "List all pets"
                    }
                }
            }
        }"#;
        let path = write_temp_json(json);
        let result = import_openapi(path.path()).unwrap();
        assert_eq!(result.collection.name, "Pet API");
        assert_eq!(result.collection.requests.len(), 1);
        assert_eq!(result.collection.requests[0].name, "listPets");
        assert_eq!(result.collection.requests[0].method, Method::Get);
        assert_eq!(result.collection.requests[0].url, "https://api.test/pets");
    }

    #[test]
    fn test_openapi_multiple_paths_methods() {
        let json = r#"{
            "openapi": "3.0.3",
            "info": {"title": "Multi", "version": "1.0"},
            "servers": [{"url": "https://api.test"}],
            "paths": {
                "/users": {
                    "get": {"operationId": "listUsers"},
                    "post": {"operationId": "createUser"}
                },
                "/users/{id}": {
                    "get": {"operationId": "getUser"},
                    "delete": {"operationId": "deleteUser"}
                }
            }
        }"#;
        let path = write_temp_json(json);
        let result = import_openapi(path.path()).unwrap();
        assert_eq!(result.collection.requests.len(), 4);
        let names: Vec<&str> = result
            .collection
            .requests
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        assert!(names.contains(&"listUsers"));
        assert!(names.contains(&"createUser"));
        assert!(names.contains(&"getUser"));
        assert!(names.contains(&"deleteUser"));
    }

    #[test]
    fn test_openapi_fallback_name() {
        let json = r#"{
            "openapi": "3.0.3",
            "info": {"title": "No OpId", "version": "1.0"},
            "servers": [{"url": "https://api.test"}],
            "paths": {
                "/health": {
                    "get": {"summary": "Health check"}
                }
            }
        }"#;
        let path = write_temp_json(json);
        let result = import_openapi(path.path()).unwrap();
        assert_eq!(result.collection.requests[0].name, "GET /health");
    }

    #[test]
    fn test_openapi_parameters() {
        let json = r#"{
            "openapi": "3.0.3",
            "info": {"title": "Params", "version": "1.0"},
            "servers": [{"url": "https://api.test"}],
            "paths": {
                "/search": {
                    "get": {
                        "operationId": "search",
                        "parameters": [
                            {"name": "q", "in": "query", "schema": {"type": "string"}},
                            {"name": "X-Request-Id", "in": "header", "schema": {"type": "string"}}
                        ]
                    }
                }
            }
        }"#;
        let path = write_temp_json(json);
        let result = import_openapi(path.path()).unwrap();
        let req = &result.collection.requests[0];
        assert_eq!(req.params.len(), 1);
        assert_eq!(req.params[0].key, "q");
        assert_eq!(req.headers.len(), 1);
        assert_eq!(req.headers[0].key, "X-Request-Id");
    }

    #[test]
    fn test_openapi_parameter_examples() {
        let json = r#"{
            "openapi": "3.0.3",
            "info": {"title": "Examples", "version": "1.0"},
            "servers": [{"url": "https://api.test"}],
            "paths": {
                "/items": {
                    "get": {
                        "operationId": "listItems",
                        "parameters": [
                            {"name": "vendor_id", "in": "query", "schema": {"type": "string"}},
                            {"name": "expand", "in": "query", "example": "parameters%2Cproduct_filter.parameters", "schema": {"type": "string"}},
                            {"name": "limit", "in": "query", "schema": {"type": "integer", "example": 25}},
                            {"name": "X-Api-Version", "in": "header", "example": "2024-01-01", "schema": {"type": "string"}}
                        ]
                    }
                }
            }
        }"#;
        let path = write_temp_json(json);
        let result = import_openapi(path.path()).unwrap();
        let req = &result.collection.requests[0];
        assert_eq!(req.params.len(), 3);
        // No example → empty
        assert_eq!(req.params[0].key, "vendor_id");
        assert_eq!(req.params[0].value, "");
        // Param-level example
        assert_eq!(req.params[1].key, "expand");
        assert_eq!(
            req.params[1].value,
            "parameters%2Cproduct_filter.parameters"
        );
        // Schema-level example (integer)
        assert_eq!(req.params[2].key, "limit");
        assert_eq!(req.params[2].value, "25");
        // Header with example
        assert_eq!(req.headers[0].key, "X-Api-Version");
        assert_eq!(req.headers[0].value, "2024-01-01");
    }

    #[test]
    fn test_openapi_request_body() {
        let json = r#"{
            "openapi": "3.0.3",
            "info": {"title": "Body", "version": "1.0"},
            "servers": [{"url": "https://api.test"}],
            "paths": {
                "/users": {
                    "post": {
                        "operationId": "createUser",
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "example": {"name": "Alice", "age": 30}
                                }
                            }
                        }
                    }
                }
            }
        }"#;
        let path = write_temp_json(json);
        let result = import_openapi(path.path()).unwrap();
        match &result.collection.requests[0].body {
            Some(Body::Raw {
                content,
                content_type,
            }) => {
                assert_eq!(*content_type, RawBodyType::Json);
                let parsed: serde_json::Value = serde_json::from_str(content).unwrap();
                assert_eq!(parsed["name"], "Alice");
            }
            other => panic!("Expected Raw JSON body, got {:?}", other),
        }
    }

    #[test]
    fn test_openapi_no_servers() {
        let json = r#"{
            "openapi": "3.0.3",
            "info": {"title": "No Server", "version": "1.0"},
            "paths": {
                "/test": {
                    "get": {"operationId": "test"}
                }
            }
        }"#;
        let path = write_temp_json(json);
        let result = import_openapi(path.path()).unwrap();
        assert_eq!(result.collection.requests[0].url, "/test");
    }

    #[test]
    fn test_openapi_yaml() {
        use std::io::Write;
        let yaml = r#"
openapi: "3.0.3"
info:
  title: YAML API
  version: "1.0"
servers:
  - url: https://yaml.test
paths:
  /items:
    get:
      operationId: listItems
"#;
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(yaml.as_bytes()).unwrap();
        file.flush().unwrap();
        let result = import_openapi(file.path()).unwrap();
        assert_eq!(result.collection.name, "YAML API");
        assert_eq!(result.collection.requests[0].url, "https://yaml.test/items");
    }

    #[test]
    fn test_openapi_swagger2_rejected() {
        let json = r#"{
            "swagger": "2.0",
            "info": {"title": "Old", "version": "1.0"},
            "paths": {}
        }"#;
        let path = write_temp_json(json);
        let result = import_openapi(path.path());
        assert!(matches!(result, Err(ImportError::UnsupportedFormat(_))));
    }

    #[test]
    fn test_openapi_empty_paths() {
        let json = r#"{
            "openapi": "3.0.3",
            "info": {"title": "Empty", "version": "1.0"},
            "paths": {}
        }"#;
        let path = write_temp_json(json);
        let result = import_openapi(path.path());
        assert!(matches!(result, Err(ImportError::EmptyImport)));
    }

    #[test]
    fn test_import_unified_curl() {
        let result = import(&ImportFormat::Curl, "curl https://example.com").unwrap();
        assert_eq!(result.collection.requests.len(), 1);
    }

    #[test]
    fn test_import_unified_postman() {
        let json = r#"{"info":{"name":"Test","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},"item":[{"name":"Req","request":{"method":"GET","url":{"raw":"https://test.com"}}}]}"#;
        let path = write_temp_json(json);
        let result = import(&ImportFormat::Postman, &path.path().to_string_lossy()).unwrap();
        assert_eq!(result.collection.name, "Test");
    }

    #[test]
    fn test_import_unified_openapi() {
        let json = r#"{"openapi":"3.0.3","info":{"title":"Test","version":"1.0"},"servers":[{"url":"https://api.test"}],"paths":{"/x":{"get":{"operationId":"x"}}}}"#;
        let path = write_temp_json(json);
        let result = import(&ImportFormat::OpenAPI, &path.path().to_string_lossy()).unwrap();
        assert_eq!(result.collection.name, "Test");
    }

    #[test]
    fn test_openapi_bearer_security() {
        let json = r#"{
            "openapi": "3.0.3",
            "info": {"title": "Secured", "version": "1.0"},
            "servers": [{"url": "https://api.test"}],
            "paths": {
                "/secret": {
                    "get": {
                        "operationId": "getSecret",
                        "security": [{"bearerAuth": []}]
                    }
                }
            },
            "components": {
                "securitySchemes": {
                    "bearerAuth": {
                        "type": "http",
                        "scheme": "bearer"
                    }
                }
            }
        }"#;
        let path = write_temp_json(json);
        let result = import_openapi(path.path()).unwrap();
        match &result.collection.requests[0].auth {
            Some(Auth::Bearer { token }) => assert_eq!(token, ""),
            other => panic!("Expected Bearer auth, got {:?}", other),
        }
    }

    #[test]
    fn test_openapi_apikey_security() {
        let json = r#"{
            "openapi": "3.0.3",
            "info": {"title": "ApiKey", "version": "1.0"},
            "servers": [{"url": "https://api.test"}],
            "paths": {
                "/data": {
                    "get": {
                        "operationId": "getData",
                        "security": [{"apiKeyAuth": []}]
                    }
                }
            },
            "components": {
                "securitySchemes": {
                    "apiKeyAuth": {
                        "type": "apiKey",
                        "in": "header",
                        "name": "X-API-Key"
                    }
                }
            }
        }"#;
        let path = write_temp_json(json);
        let result = import_openapi(path.path()).unwrap();
        match &result.collection.requests[0].auth {
            Some(Auth::ApiKey {
                key,
                value,
                location,
            }) => {
                assert_eq!(key, "X-API-Key");
                assert_eq!(value, "");
                assert_eq!(*location, crate::types::ApiKeyLocation::Header);
            }
            other => panic!("Expected ApiKey auth, got {:?}", other),
        }
    }
}
