use crate::collection::slugify;
use crate::command::CurlCommandBuilder;
use crate::config::config_dir;
use crate::types::{ApiKeyLocation, Auth, Body, Collection, OAuth2Grant, Request};

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
        &[
            ExportFormat::Curl,
            ExportFormat::PostmanV21,
            ExportFormat::OpenApi3,
        ]
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
            Auth::ApiKey {
                key,
                value,
                location,
            } => match location {
                ApiKeyLocation::Header => {
                    builder = builder.header(key, value);
                }
                ApiKeyLocation::Query => {
                    builder = builder.query_param(key, value);
                }
            },
            Auth::None => {}
            Auth::Digest {
                username, password, ..
            } => {
                builder = builder.digest_auth(username, password);
            }
            Auth::OAuth1 { .. } => {
                // OAuth 1.0 requires runtime signing; can't compute at export time
                builder = builder.header("Authorization", "OAuth <requires runtime signing>");
            }
            Auth::OAuth2 { access_token, .. } => {
                if !access_token.is_empty() {
                    builder = builder.header("Authorization", &format!("Bearer {}", access_token));
                }
            }
            Auth::AwsV4 {
                region, service, ..
            } => {
                // curl supports --aws-sigv4 natively since 7.75.0
                // Use a comment-style header since CurlCommandBuilder may not have raw arg support
                builder = builder.header(
                    "Authorization",
                    &format!(
                        "AWS4-HMAC-SHA256 (aws-sigv4 provider:amz:{}:{})",
                        region, service
                    ),
                );
            }
            Auth::Asap { .. } => {
                builder = builder.header("Authorization", "Bearer <requires runtime JWT signing>");
            }
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
        Some(Auth::ApiKey {
            key,
            value,
            location,
        }) => {
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
        Some(Auth::Digest {
            username, password, ..
        }) => Some(serde_json::json!({
            "type": "digest",
            "digest": [
                {"key": "username", "value": username, "type": "string"},
                {"key": "password", "value": password, "type": "string"},
            ]
        })),
        Some(Auth::OAuth1 {
            consumer_key,
            consumer_secret,
            access_token,
            token_secret,
            signature_method,
            ..
        }) => {
            let sig_method = match signature_method {
                crate::types::OAuth1SignatureMethod::HmacSha1 => "HMAC-SHA1",
                crate::types::OAuth1SignatureMethod::HmacSha256 => "HMAC-SHA256",
                crate::types::OAuth1SignatureMethod::Plaintext => "PLAINTEXT",
            };
            Some(serde_json::json!({
                "type": "oauth1",
                "oauth1": [
                    {"key": "consumerKey", "value": consumer_key, "type": "string"},
                    {"key": "consumerSecret", "value": consumer_secret, "type": "string"},
                    {"key": "token", "value": access_token, "type": "string"},
                    {"key": "tokenSecret", "value": token_secret, "type": "string"},
                    {"key": "signatureMethod", "value": sig_method, "type": "string"},
                ]
            }))
        }
        Some(Auth::OAuth2 {
            grant,
            scope,
            access_token,
            ..
        }) => {
            let grant_type = match grant {
                crate::types::OAuth2Grant::AuthorizationCode { .. } => "authorization_code",
                crate::types::OAuth2Grant::Pkce { .. } => "authorization_code_with_pkce",
                crate::types::OAuth2Grant::ClientCredentials { .. } => "client_credentials",
                crate::types::OAuth2Grant::Password { .. } => "password",
            };
            Some(serde_json::json!({
                "type": "oauth2",
                "oauth2": [
                    {"key": "grant_type", "value": grant_type, "type": "string"},
                    {"key": "accessToken", "value": access_token, "type": "string"},
                    {"key": "scope", "value": scope, "type": "string"},
                ]
            }))
        }
        Some(Auth::AwsV4 {
            access_key,
            secret_key,
            region,
            service,
            ..
        }) => Some(serde_json::json!({
            "type": "awsv4",
            "awsv4": [
                {"key": "accessKey", "value": access_key, "type": "string"},
                {"key": "secretKey", "value": secret_key, "type": "string"},
                {"key": "region", "value": region, "type": "string"},
                {"key": "service", "value": service, "type": "string"},
            ]
        })),
        Some(Auth::Asap {
            issuer, audience, ..
        }) => Some(serde_json::json!({
            "type": "bearer",
            "bearer": [
                {"key": "token", "value": format!("<ASAP JWT - iss:{} aud:{}>", issuer, audience), "type": "string"},
            ]
        })),
        Some(Auth::None) | None => None,
    }
}

/// Export a single request as an OpenAPI 3.0.3 JSON object.
pub fn export_openapi_request(request: &Request) -> serde_json::Value {
    let (server, path) = parse_url_parts(&request.url);
    let method_key = request.method.to_string().to_lowercase();
    let operation = openapi_operation(request);

    let mut paths = serde_json::Map::new();
    let mut path_obj = serde_json::Map::new();
    path_obj.insert(method_key, operation);
    paths.insert(path, serde_json::Value::Object(path_obj));

    let mut spec = serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": request.name,
            "version": "1.0.0"
        },
        "servers": [{"url": server}],
        "paths": paths
    });

    let security_schemes = collect_security_schemes(std::slice::from_ref(request));
    if !security_schemes.is_empty() {
        spec["components"] = serde_json::json!({"securitySchemes": security_schemes});
    }

    spec
}

/// Export a collection as an OpenAPI 3.0.3 JSON object.
pub fn export_openapi_collection(collection: &Collection) -> serde_json::Value {
    let mut path_map: std::collections::BTreeMap<
        String,
        serde_json::Map<String, serde_json::Value>,
    > = std::collections::BTreeMap::new();
    let mut server = String::new();

    for request in &collection.requests {
        let (srv, path) = parse_url_parts(&request.url);
        if server.is_empty() {
            server = srv;
        }
        let method_key = request.method.to_string().to_lowercase();
        let operation = openapi_operation(request);
        path_map
            .entry(path)
            .or_default()
            .insert(method_key, operation);
    }

    let mut paths = serde_json::Map::new();
    for (path, methods) in path_map {
        paths.insert(path, serde_json::Value::Object(methods));
    }

    let mut spec = serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": collection.name,
            "version": "1.0.0"
        },
        "servers": if server.is_empty() { serde_json::json!([]) } else { serde_json::json!([{"url": server}]) },
        "paths": paths
    });

    let security_schemes = collect_security_schemes(&collection.requests);
    if !security_schemes.is_empty() {
        spec["components"] = serde_json::json!({"securitySchemes": security_schemes});
    }

    spec
}

fn parse_url_parts(url: &str) -> (String, String) {
    let url_no_query = url.split('?').next().unwrap_or(url);
    if let Some(scheme_end) = url_no_query.find("://") {
        let after_scheme = &url_no_query[scheme_end + 3..];
        if let Some(slash_pos) = after_scheme.find('/') {
            let server = &url_no_query[..scheme_end + 3 + slash_pos];
            let path = &after_scheme[slash_pos..];
            return (server.to_string(), path.to_string());
        }
        return (url_no_query.to_string(), "/".to_string());
    }
    (String::new(), url_no_query.to_string())
}

fn openapi_operation(request: &Request) -> serde_json::Value {
    let mut operation = serde_json::json!({
        "summary": request.name,
        "responses": { "200": { "description": "Successful response" } }
    });

    let mut parameters: Vec<serde_json::Value> = Vec::new();
    for header in &request.headers {
        if header.enabled {
            parameters.push(serde_json::json!({
                "name": header.key, "in": "header",
                "schema": { "type": "string" }, "example": header.value
            }));
        }
    }
    for param in &request.params {
        if param.enabled {
            parameters.push(serde_json::json!({
                "name": param.key, "in": "query",
                "schema": { "type": "string" }, "example": param.value
            }));
        }
    }
    if !parameters.is_empty() {
        operation["parameters"] = serde_json::Value::Array(parameters);
    }

    if let Some(body) = &request.body {
        match body {
            Body::Json { content } => {
                operation["requestBody"] = serde_json::json!({
                    "content": { "application/json": {
                        "schema": { "type": "object" },
                        "example": serde_json::from_str::<serde_json::Value>(content)
                            .unwrap_or_else(|_| serde_json::Value::String(content.clone()))
                    }}
                });
            }
            Body::Text { content } => {
                operation["requestBody"] = serde_json::json!({
                    "content": { "text/plain": {
                        "schema": { "type": "string" }, "example": content
                    }}
                });
            }
            Body::Form { fields } => {
                let mut properties = serde_json::Map::new();
                for field in fields.iter().filter(|f| f.enabled) {
                    properties.insert(
                        field.key.clone(),
                        serde_json::json!({"type": "string", "example": field.value}),
                    );
                }
                operation["requestBody"] = serde_json::json!({
                    "content": { "application/x-www-form-urlencoded": {
                        "schema": { "type": "object", "properties": properties }
                    }}
                });
            }
            Body::Multipart { parts } => {
                let mut properties = serde_json::Map::new();
                for part in parts {
                    if part.file_path.is_some() {
                        properties.insert(
                            part.name.clone(),
                            serde_json::json!({"type": "string", "format": "binary"}),
                        );
                    } else {
                        properties.insert(part.name.clone(), serde_json::json!({"type": "string", "example": part.value.as_deref().unwrap_or("")}));
                    }
                }
                operation["requestBody"] = serde_json::json!({
                    "content": { "multipart/form-data": {
                        "schema": { "type": "object", "properties": properties }
                    }}
                });
            }
            Body::None => {}
        }
    }

    if let Some(auth) = &request.auth {
        let security_name = match auth {
            Auth::Bearer { .. } => Some("bearerAuth"),
            Auth::Basic { .. } => Some("basicAuth"),
            Auth::ApiKey { .. } => Some("apiKeyAuth"),
            Auth::Digest { .. } => Some("digestAuth"),
            Auth::OAuth1 { .. } => Some("oauth1Auth"),
            Auth::OAuth2 { .. } => Some("oauth2Auth"),
            Auth::AwsV4 { .. } => Some("awsV4Auth"),
            Auth::Asap { .. } => Some("asapAuth"),
            Auth::None => None,
        };
        if let Some(name) = security_name {
            operation["security"] = serde_json::json!([{name: []}]);
        }
    }

    operation
}

fn collect_security_schemes(requests: &[Request]) -> serde_json::Map<String, serde_json::Value> {
    let mut schemes = serde_json::Map::new();
    for request in requests {
        if let Some(auth) = &request.auth {
            match auth {
                Auth::Bearer { .. } => {
                    schemes
                        .entry("bearerAuth")
                        .or_insert_with(|| serde_json::json!({"type": "http", "scheme": "bearer"}));
                }
                Auth::Basic { .. } => {
                    schemes
                        .entry("basicAuth")
                        .or_insert_with(|| serde_json::json!({"type": "http", "scheme": "basic"}));
                }
                Auth::ApiKey { key, location, .. } => {
                    let loc = match location {
                        ApiKeyLocation::Header => "header",
                        ApiKeyLocation::Query => "query",
                    };
                    schemes.entry("apiKeyAuth").or_insert_with(
                        || serde_json::json!({"type": "apiKey", "name": key, "in": loc}),
                    );
                }
                Auth::None => {}
                Auth::Digest { .. } => {
                    schemes.insert(
                        "digestAuth".to_string(),
                        serde_json::json!({"type": "http", "scheme": "digest"}),
                    );
                }
                Auth::OAuth1 { .. } => {
                    schemes.insert(
                        "oauth1Auth".to_string(),
                        serde_json::json!({
                            "type": "apiKey",
                            "in": "header",
                            "name": "Authorization",
                            "description": "OAuth 1.0 (not natively supported in OpenAPI 3.0)"
                        }),
                    );
                }
                Auth::OAuth2 { grant, .. } => {
                    let flow = match grant {
                        OAuth2Grant::AuthorizationCode {
                            auth_url,
                            token_url,
                            ..
                        }
                        | OAuth2Grant::Pkce {
                            auth_url,
                            token_url,
                            ..
                        } => {
                            serde_json::json!({
                                "authorizationCode": {
                                    "authorizationUrl": auth_url,
                                    "tokenUrl": token_url,
                                    "scopes": {}
                                }
                            })
                        }
                        OAuth2Grant::ClientCredentials { token_url, .. } => {
                            serde_json::json!({
                                "clientCredentials": {
                                    "tokenUrl": token_url,
                                    "scopes": {}
                                }
                            })
                        }
                        OAuth2Grant::Password { token_url, .. } => {
                            serde_json::json!({
                                "password": {
                                    "tokenUrl": token_url,
                                    "scopes": {}
                                }
                            })
                        }
                    };
                    schemes.insert(
                        "oauth2Auth".to_string(),
                        serde_json::json!({"type": "oauth2", "flows": flow}),
                    );
                }
                Auth::AwsV4 { .. } => {
                    schemes.insert(
                        "awsV4Auth".to_string(),
                        serde_json::json!({
                            "type": "apiKey",
                            "in": "header",
                            "name": "Authorization",
                            "description": "AWS Signature V4"
                        }),
                    );
                }
                Auth::Asap { .. } => {
                    schemes.insert(
                        "asapAuth".to_string(),
                        serde_json::json!({"type": "http", "scheme": "bearer", "bearerFormat": "ASAP"}),
                    );
                }
            }
        }
    }
    schemes
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
    use crate::types::{
        ClientAuthentication, Collection, DigestAlgorithm, FormField, Header, Method, OAuth2Grant,
        Param, Variable,
    };
    use std::collections::HashMap;

    fn make_request() -> Request {
        Request {
            id: uuid::Uuid::new_v4(),
            name: "Get Users".to_string(),
            method: Method::Get,
            url: "https://api.example.com/users".to_string(),
            headers: vec![
                Header {
                    key: "Accept".to_string(),
                    value: "application/json".to_string(),
                    enabled: true,
                },
                Header {
                    key: "X-Debug".to_string(),
                    value: "true".to_string(),
                    enabled: false,
                },
            ],
            params: vec![Param {
                key: "page".to_string(),
                value: "1".to_string(),
                enabled: true,
            }],
            body: None,
            auth: Some(Auth::Bearer {
                token: "my-secret-token".to_string(),
            }),
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
            body: Some(Body::Json {
                content: r#"{"name":"Alice"}"#.to_string(),
            }),
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
                    FormField {
                        key: "user".to_string(),
                        value: "alice".to_string(),
                        enabled: true,
                    },
                    FormField {
                        key: "disabled".to_string(),
                        value: "skip".to_string(),
                        enabled: false,
                    },
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
            auth: Some(Auth::Basic {
                username: "user".to_string(),
                password: "pass".to_string(),
            }),
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
                m.insert(
                    "base_url".to_string(),
                    Variable {
                        value: "https://api.example.com".to_string(),
                        secret: false,
                    },
                );
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
            headers: vec![],
            params: vec![],
            body: Some(Body::Json {
                content: r#"{"key":"value"}"#.to_string(),
            }),
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
            headers: vec![],
            params: vec![],
            body: Some(Body::Form {
                fields: vec![FormField {
                    key: "user".to_string(),
                    value: "alice".to_string(),
                    enabled: true,
                }],
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
            headers: vec![],
            params: vec![],
            body: None,
            auth: Some(Auth::Basic {
                username: "user".to_string(),
                password: "pass".to_string(),
            }),
        };
        let json = export_postman_request(&req);
        let auth = &json["item"][0]["request"]["auth"];
        assert_eq!(auth["type"].as_str().unwrap(), "basic");
        let basic = auth["basic"].as_array().unwrap();
        assert!(basic
            .iter()
            .any(|v| v["key"] == "username" && v["value"] == "user"));
        assert!(basic
            .iter()
            .any(|v| v["key"] == "password" && v["value"] == "pass"));
    }

    #[test]
    fn test_export_openapi_single_request() {
        let req = make_request();
        let json = export_openapi_request(&req);
        assert_eq!(json["openapi"].as_str().unwrap(), "3.0.3");
        assert_eq!(json["info"]["title"].as_str().unwrap(), "Get Users");
        assert_eq!(json["info"]["version"].as_str().unwrap(), "1.0.0");
        let paths = json["paths"].as_object().unwrap();
        assert_eq!(paths.len(), 1);
        let (_, path_obj) = paths.iter().next().unwrap();
        let operation = &path_obj["get"];
        assert!(operation.is_object());
        let params = operation["parameters"].as_array().unwrap();
        assert_eq!(params.len(), 2);
    }

    #[test]
    fn test_export_openapi_collection_groups_by_path() {
        let req1 = Request {
            id: uuid::Uuid::new_v4(),
            name: "List Users".to_string(),
            method: Method::Get,
            url: "https://api.example.com/users".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: None,
        };
        let req2 = Request {
            id: uuid::Uuid::new_v4(),
            name: "Create User".to_string(),
            method: Method::Post,
            url: "https://api.example.com/users".to_string(),
            headers: vec![],
            params: vec![],
            body: Some(Body::Json {
                content: r#"{"name":"Alice"}"#.to_string(),
            }),
            auth: None,
        };
        let collection = Collection {
            id: uuid::Uuid::new_v4(),
            name: "User API".to_string(),
            variables: HashMap::new(),
            requests: vec![req1, req2],
        };
        let json = export_openapi_collection(&collection);
        let paths = json["paths"].as_object().unwrap();
        assert_eq!(paths.len(), 1);
        let users_path = &paths["/users"];
        assert!(users_path["get"].is_object());
        assert!(users_path["post"].is_object());
    }

    #[test]
    fn test_export_openapi_request_body() {
        let req = Request {
            id: uuid::Uuid::new_v4(),
            name: "Create".to_string(),
            method: Method::Post,
            url: "https://example.com/api/items".to_string(),
            headers: vec![],
            params: vec![],
            body: Some(Body::Json {
                content: r#"{"key":"val"}"#.to_string(),
            }),
            auth: None,
        };
        let json = export_openapi_request(&req);
        let op = &json["paths"]["/api/items"]["post"];
        let content = &op["requestBody"]["content"]["application/json"];
        assert!(content.is_object());
    }

    #[test]
    fn test_export_openapi_bearer_auth() {
        let req = Request {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            method: Method::Get,
            url: "https://example.com/api".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: Some(Auth::Bearer {
                token: "tok".to_string(),
            }),
        };
        let json = export_openapi_request(&req);
        let schemes = json["components"]["securitySchemes"].as_object().unwrap();
        assert!(schemes.contains_key("bearerAuth"));
        assert_eq!(schemes["bearerAuth"]["scheme"].as_str().unwrap(), "bearer");
    }

    #[test]
    fn test_export_openapi_servers_from_url() {
        let req = Request {
            id: uuid::Uuid::new_v4(),
            name: "Test".to_string(),
            method: Method::Get,
            url: "https://api.example.com/v1/users".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: None,
        };
        let json = export_openapi_request(&req);
        let servers = json["servers"].as_array().unwrap();
        assert_eq!(
            servers[0]["url"].as_str().unwrap(),
            "https://api.example.com"
        );
    }

    #[test]
    fn test_export_curl_digest_auth() {
        let request = Request {
            id: uuid::Uuid::new_v4(),
            name: "Digest Test".to_string(),
            method: Method::Get,
            url: "https://example.com".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: Some(Auth::Digest {
                username: "user".to_string(),
                password: "pass".to_string(),
                realm: String::new(),
                nonce: String::new(),
                algorithm: DigestAlgorithm::MD5,
                qop: String::new(),
                nonce_count: String::new(),
                client_nonce: String::new(),
                opaque: String::new(),
            }),
        };
        let result = export_curl(&request, &[]);
        assert!(result.contains("--digest"));
        assert!(result.contains("-u"));
        assert!(result.contains("user:pass"));
    }

    #[test]
    fn test_export_curl_oauth2_with_token() {
        let request = Request {
            id: uuid::Uuid::new_v4(),
            name: "OAuth2 Test".to_string(),
            method: Method::Get,
            url: "https://api.example.com".to_string(),
            headers: vec![],
            params: vec![],
            body: None,
            auth: Some(Auth::OAuth2 {
                grant: OAuth2Grant::AuthorizationCode {
                    auth_url: "https://auth.example.com/authorize".to_string(),
                    token_url: "https://auth.example.com/token".to_string(),
                    client_id: "cid".to_string(),
                    client_secret: "csecret".to_string(),
                },
                token_name: String::new(),
                callback_url: String::new(),
                scope: "read".to_string(),
                state: String::new(),
                client_authentication: ClientAuthentication::BasicHeader,
                access_token: "my-token-123".to_string(),
                refresh_token: String::new(),
            }),
        };
        let result = export_curl(&request, &[]);
        assert!(result.contains("Bearer my-token-123"));
    }
}
