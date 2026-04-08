use crate::types::{Body, BodyType, Header};

/// Compute auto-generated headers based on body type.
/// Returns empty vec if user already has an enabled Content-Type header
/// or if the body type has no associated content type.
pub fn resolve_auto_headers(body: &Body, user_headers: &[Header]) -> Vec<Header> {
    // Check if user already has an enabled Content-Type header (case-insensitive)
    let has_content_type = user_headers
        .iter()
        .any(|h| h.enabled && h.key.eq_ignore_ascii_case("content-type"));

    if has_content_type {
        return Vec::new();
    }

    let body_type = BodyType::from(body);
    match body_type.content_type() {
        Some(ct) => vec![Header {
            key: "Content-Type".to_string(),
            value: ct.to_string(),
            enabled: true,
        }],
        None => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RawBodyType;

    #[test]
    fn auto_content_type_for_json_body() {
        let body = Body::Raw {
            content: "{}".to_string(),
            content_type: RawBodyType::Json,
        };
        let headers = resolve_auto_headers(&body, &[]);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].key, "Content-Type");
        assert_eq!(headers[0].value, "application/json");
        assert!(headers[0].enabled);
    }

    #[test]
    fn auto_content_type_for_xml_body() {
        let body = Body::Raw {
            content: "<root/>".to_string(),
            content_type: RawBodyType::Xml,
        };
        let headers = resolve_auto_headers(&body, &[]);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].value, "application/xml");
    }

    #[test]
    fn auto_content_type_for_form_body() {
        let body = Body::Form { fields: vec![] };
        let headers = resolve_auto_headers(&body, &[]);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].value, "application/x-www-form-urlencoded");
    }

    #[test]
    fn auto_content_type_for_graphql_body() {
        let body = Body::GraphQL {
            query: "{ users }".to_string(),
            variables: String::new(),
        };
        let headers = resolve_auto_headers(&body, &[]);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].value, "application/json");
    }

    #[test]
    fn no_auto_content_type_for_binary() {
        let body = Body::Binary {
            file_path: "/tmp/file.bin".to_string(),
        };
        let headers = resolve_auto_headers(&body, &[]);
        assert!(headers.is_empty());
    }

    #[test]
    fn no_auto_content_type_for_none() {
        let body = Body::None;
        let headers = resolve_auto_headers(&body, &[]);
        assert!(headers.is_empty());
    }

    #[test]
    fn suppressed_when_user_has_content_type() {
        let body = Body::Raw {
            content: "{}".to_string(),
            content_type: RawBodyType::Json,
        };
        let user_headers = vec![Header {
            key: "Content-Type".to_string(),
            value: "text/plain".to_string(),
            enabled: true,
        }];
        let headers = resolve_auto_headers(&body, &user_headers);
        assert!(headers.is_empty());
    }

    #[test]
    fn suppressed_case_insensitive_content_type() {
        let body = Body::Raw {
            content: "{}".to_string(),
            content_type: RawBodyType::Json,
        };
        let user_headers = vec![Header {
            key: "content-type".to_string(),
            value: "text/plain".to_string(),
            enabled: true,
        }];
        let headers = resolve_auto_headers(&body, &user_headers);
        assert!(headers.is_empty());
    }

    #[test]
    fn not_suppressed_when_user_content_type_disabled() {
        let body = Body::Raw {
            content: "{}".to_string(),
            content_type: RawBodyType::Json,
        };
        let user_headers = vec![Header {
            key: "Content-Type".to_string(),
            value: "text/plain".to_string(),
            enabled: false,
        }];
        let headers = resolve_auto_headers(&body, &user_headers);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].value, "application/json");
    }
}
