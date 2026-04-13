use std::fmt;

use crate::types::Collection;

/// Supported import formats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportFormat {
    Curl,
    Postman,
    OpenAPI,
}

impl ImportFormat {
    pub fn all() -> &'static [ImportFormat] {
        &[ImportFormat::Curl, ImportFormat::Postman, ImportFormat::OpenAPI]
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

/// Detect import format from file content.
pub fn detect_format(content: &str) -> Result<ImportFormat, ImportError> {
    let trimmed = content.trim();
    if trimmed.starts_with("curl ") || trimmed.starts_with("curl.exe ") {
        return Ok(ImportFormat::Curl);
    }
    // Try JSON first
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if json.get("info")
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
        assert_eq!(detect_format("curl https://example.com").unwrap(), ImportFormat::Curl);
        assert_eq!(detect_format("  curl -X POST https://api.test").unwrap(), ImportFormat::Curl);
    }

    #[test]
    fn test_detect_format_curl_exe() {
        assert_eq!(detect_format("curl.exe https://example.com").unwrap(), ImportFormat::Curl);
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
}
