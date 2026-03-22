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
pub struct HistoryEntry {
    pub id: uuid::Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<uuid::Uuid>,
    pub request_name: String,
    pub method: Method,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
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

/// Parsed response from a curl execution
#[derive(Debug, Clone)]
pub struct CurlResponse {
    pub status_code: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub timing: ResponseTiming,
    pub raw_command: String,
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
    fn test_history_entry_roundtrip() {
        let entry = HistoryEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            collection_id: None,
            request_name: "Get Users".to_string(),
            method: Method::Get,
            url: "https://api.example.com/users".to_string(),
            status_code: Some(200),
            duration_ms: Some(142),
            environment: Some("Development".to_string()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: HistoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.request_name, "Get Users");
        assert_eq!(deserialized.status_code, Some(200));
    }
}
