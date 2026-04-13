use std::fmt;

use crate::types::{
    Auth, Body, Collection, FormField, Header, Method, MultipartPart, Param, RawBodyType, Request,
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
}
