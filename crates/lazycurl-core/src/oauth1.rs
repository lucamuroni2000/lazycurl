use base64::Engine;
use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::Sha256;
use url::Url;

use crate::types::OAuth1SignatureMethod;

/// RFC 5849 Section 3.6 percent-encoding.
/// Encodes all bytes except the unreserved set (A-Z, a-z, 0-9, `-`, `.`, `_`, `~`).
pub fn percent_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len() * 2);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

/// Normalize a URL per RFC 5849 Section 3.4.1.2:
/// - Lowercase scheme and host
/// - Remove default ports (80 for http, 443 for https)
/// - Strip query string and fragment
/// - Reconstruct as `scheme://host[:port]/path`
fn normalize_base_url(raw_url: &str) -> String {
    match Url::parse(raw_url) {
        Ok(parsed) => {
            let scheme = parsed.scheme(); // already lowercased by url crate
            let host = parsed.host_str().unwrap_or(""); // already lowercased by url crate
            let port = parsed.port();
            let path = parsed.path();

            // Omit default ports
            let include_port = match (scheme, port) {
                ("http", Some(80)) | ("https", Some(443)) => false,
                (_, Some(_)) => true,
                _ => false,
            };

            if include_port {
                format!("{}://{}:{}{}", scheme, host, port.unwrap(), path)
            } else {
                format!("{}://{}{}", scheme, host, path)
            }
        }
        Err(_) => raw_url.to_string(),
    }
}

/// RFC 5849 Section 3.4.1: Build the signature base string.
/// `METHOD&percent_encode(url)&percent_encode(normalized_params)`
/// Params are percent-encoded, sorted by key then value, joined with `&`.
pub fn signature_base_string(method: &str, url: &str, params: &[(&str, &str)]) -> String {
    // Percent-encode each key and value, then sort
    let mut encoded_params: Vec<(String, String)> = params
        .iter()
        .map(|(k, v)| (percent_encode(k), percent_encode(v)))
        .collect();
    encoded_params.sort();

    // Join sorted params with &
    let normalized: String = encoded_params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    let base_url = normalize_base_url(url);

    format!(
        "{}&{}&{}",
        percent_encode(&method.to_uppercase()),
        percent_encode(&base_url),
        percent_encode(&normalized)
    )
}

/// HMAC-SHA1 signing per RFC 5849 Section 3.4.2.
/// Key = `percent_encode(consumer_secret)&percent_encode(token_secret)`.
/// Returns base64-encoded signature.
pub fn sign_hmac_sha1(base_string: &str, consumer_secret: &str, token_secret: &str) -> String {
    let key = format!(
        "{}&{}",
        percent_encode(consumer_secret),
        percent_encode(token_secret)
    );
    let mut mac =
        Hmac::<Sha1>::new_from_slice(key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(base_string.as_bytes());
    let result = mac.finalize().into_bytes();
    base64::engine::general_purpose::STANDARD.encode(result)
}

/// HMAC-SHA256 signing (same as HMAC-SHA1 but using SHA-256).
/// Key = `percent_encode(consumer_secret)&percent_encode(token_secret)`.
/// Returns base64-encoded signature.
pub fn sign_hmac_sha256(base_string: &str, consumer_secret: &str, token_secret: &str) -> String {
    let key = format!(
        "{}&{}",
        percent_encode(consumer_secret),
        percent_encode(token_secret)
    );
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key.as_bytes()).expect("HMAC can take key of any size");
    mac.update(base_string.as_bytes());
    let result = mac.finalize().into_bytes();
    base64::engine::general_purpose::STANDARD.encode(result)
}

/// PLAINTEXT signing per RFC 5849 Section 3.4.4.
/// Returns `percent_encode(consumer_secret)&percent_encode(token_secret)`.
pub fn sign_plaintext(consumer_secret: &str, token_secret: &str) -> String {
    format!(
        "{}&{}",
        percent_encode(consumer_secret),
        percent_encode(token_secret)
    )
}

/// Build the complete `OAuth ...` Authorization header value.
///
/// Collects all `oauth_*` params, computes the signature using the appropriate method,
/// then formats as `OAuth key="value", ...` pairs.
#[allow(clippy::too_many_arguments)]
pub fn build_authorization_header(
    method: &str,
    url: &str,
    signature_method: &OAuth1SignatureMethod,
    consumer_key: &str,
    consumer_secret: &str,
    access_token: &str,
    token_secret: &str,
    timestamp: &str,
    nonce: &str,
    version: &str,
    realm: &str,
    // Body hash (oauth_body_hash) not yet implemented
    include_body_hash: bool,
    extra_params: &[(&str, &str)],
) -> String {
    let _ = include_body_hash; // Not yet implemented
    let sig_method_str = match signature_method {
        OAuth1SignatureMethod::HmacSha1 => "HMAC-SHA1",
        OAuth1SignatureMethod::HmacSha256 => "HMAC-SHA256",
        OAuth1SignatureMethod::Plaintext => "PLAINTEXT",
    };

    // Collect all oauth params for signature base string
    let mut all_params: Vec<(&str, &str)> = vec![
        ("oauth_consumer_key", consumer_key),
        ("oauth_nonce", nonce),
        ("oauth_signature_method", sig_method_str),
        ("oauth_timestamp", timestamp),
        ("oauth_version", version),
    ];
    // Only include oauth_token for three-legged OAuth (non-empty access_token)
    if !access_token.is_empty() {
        all_params.push(("oauth_token", access_token));
    }
    // Add extra params (e.g., request query params or body params)
    all_params.extend_from_slice(extra_params);

    // Compute signature (skip base string computation for PLAINTEXT)
    let signature = match signature_method {
        OAuth1SignatureMethod::HmacSha1 => {
            let base_string = signature_base_string(method, url, &all_params);
            sign_hmac_sha1(&base_string, consumer_secret, token_secret)
        }
        OAuth1SignatureMethod::HmacSha256 => {
            let base_string = signature_base_string(method, url, &all_params);
            sign_hmac_sha256(&base_string, consumer_secret, token_secret)
        }
        OAuth1SignatureMethod::Plaintext => sign_plaintext(consumer_secret, token_secret),
    };

    // Build the header value
    // RFC 5849 Section 3.5.1: realm is a quoted-string, NOT percent-encoded
    let mut parts: Vec<String> = Vec::new();
    if !realm.is_empty() {
        parts.push(format!("realm=\"{}\"", realm));
    }
    parts.push(format!(
        "oauth_consumer_key=\"{}\"",
        percent_encode(consumer_key)
    ));
    parts.push(format!("oauth_nonce=\"{}\"", percent_encode(nonce)));
    parts.push(format!(
        "oauth_signature=\"{}\"",
        percent_encode(&signature)
    ));
    parts.push(format!(
        "oauth_signature_method=\"{}\"",
        percent_encode(sig_method_str)
    ));
    parts.push(format!("oauth_timestamp=\"{}\"", percent_encode(timestamp)));
    if !access_token.is_empty() {
        parts.push(format!("oauth_token=\"{}\"", percent_encode(access_token)));
    }
    parts.push(format!("oauth_version=\"{}\"", percent_encode(version)));

    format!("OAuth {}", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- percent_encode tests ----

    #[test]
    fn test_percent_encode_spaces_and_plus() {
        assert_eq!(
            percent_encode("Ladies + Gentlemen"),
            "Ladies%20%2B%20Gentlemen"
        );
    }

    #[test]
    fn test_percent_encode_exclamation() {
        assert_eq!(
            percent_encode("An encoded string!"),
            "An%20encoded%20string%21"
        );
    }

    #[test]
    fn test_percent_encode_unreserved_passthrough() {
        assert_eq!(percent_encode("unreserved-._~"), "unreserved-._~");
    }

    #[test]
    fn test_percent_encode_empty() {
        assert_eq!(percent_encode(""), "");
    }

    #[test]
    fn test_percent_encode_alphanumeric() {
        assert_eq!(percent_encode("abc123XYZ"), "abc123XYZ");
    }

    // ---- signature_base_string tests ----

    #[test]
    fn test_signature_base_string_rfc5849_example() {
        let method = "GET";
        let url = "http://photos.example.net/photos";
        let params = vec![
            ("oauth_consumer_key", "dpf43f3p2l4k3l03"),
            ("oauth_nonce", "kllo9940pd9333jh"),
            ("oauth_signature_method", "HMAC-SHA1"),
            ("oauth_timestamp", "1191242096"),
            ("oauth_token", "nnch734d00sl2jdk"),
            ("oauth_version", "1.0"),
            ("size", "original"),
            ("file", "vacation.jpg"),
        ];
        let expected = "GET&http%3A%2F%2Fphotos.example.net%2Fphotos&file%3Dvacation.jpg%26oauth_consumer_key%3Ddpf43f3p2l4k3l03%26oauth_nonce%3Dkllo9940pd9333jh%26oauth_signature_method%3DHMAC-SHA1%26oauth_timestamp%3D1191242096%26oauth_token%3Dnnch734d00sl2jdk%26oauth_version%3D1.0%26size%3Doriginal";
        assert_eq!(signature_base_string(method, url, &params), expected);
    }

    #[test]
    fn test_signature_base_string_sorts_params() {
        let result = signature_base_string("POST", "http://example.com", &[("z", "1"), ("a", "2")]);
        // "a" should come before "z" in the normalized params
        assert!(result.contains("a%3D2%26z%3D1"));
    }

    // ---- sign_hmac_sha1 tests ----

    #[test]
    fn test_sign_hmac_sha1_rfc5849_example() {
        let base_string = "GET&http%3A%2F%2Fphotos.example.net%2Fphotos&file%3Dvacation.jpg%26oauth_consumer_key%3Ddpf43f3p2l4k3l03%26oauth_nonce%3Dkllo9940pd9333jh%26oauth_signature_method%3DHMAC-SHA1%26oauth_timestamp%3D1191242096%26oauth_token%3Dnnch734d00sl2jdk%26oauth_version%3D1.0%26size%3Doriginal";
        let signature = sign_hmac_sha1(base_string, "kd94hf93k423kf44", "pfkkdhi9sl3r4s00");
        assert_eq!(signature, "tR3+Ty81lMeYAr/Fid0kMTYa/WM=");
    }

    // ---- sign_hmac_sha256 tests ----

    #[test]
    fn test_sign_hmac_sha256_produces_valid_base64() {
        let base_string = "GET&http%3A%2F%2Fexample.com&oauth_consumer_key%3Dkey";
        let signature = sign_hmac_sha256(base_string, "consumer_secret", "token_secret");
        assert!(!signature.is_empty());
        // Verify it's valid base64
        assert!(base64::engine::general_purpose::STANDARD
            .decode(&signature)
            .is_ok());
    }

    // ---- sign_plaintext tests ----

    #[test]
    fn test_sign_plaintext() {
        assert_eq!(
            sign_plaintext("consumer_secret", "token_secret"),
            "consumer_secret&token_secret"
        );
    }

    #[test]
    fn test_sign_plaintext_with_special_chars() {
        // Special characters should be percent-encoded
        assert_eq!(sign_plaintext("sec&ret", "tok!en"), "sec%26ret&tok%21en");
    }

    // ---- build_authorization_header tests ----

    #[test]
    fn test_build_authorization_header_starts_with_oauth() {
        let header = build_authorization_header(
            "GET",
            "http://example.com/resource",
            &OAuth1SignatureMethod::HmacSha1,
            "consumer_key_123",
            "consumer_secret_456",
            "access_token_789",
            "token_secret_abc",
            "1234567890",
            "testnonce",
            "1.0",
            "",
            false,
            &[],
        );
        assert!(header.starts_with("OAuth "));
    }

    #[test]
    fn test_build_authorization_header_contains_required_params() {
        let header = build_authorization_header(
            "GET",
            "http://example.com/resource",
            &OAuth1SignatureMethod::HmacSha1,
            "consumer_key_123",
            "consumer_secret_456",
            "access_token_789",
            "token_secret_abc",
            "1234567890",
            "testnonce",
            "1.0",
            "",
            false,
            &[],
        );
        assert!(header.contains("oauth_consumer_key=\"consumer_key_123\""));
        assert!(header.contains("oauth_token=\"access_token_789\""));
        assert!(header.contains("oauth_signature="));
        assert!(header.contains("oauth_signature_method=\"HMAC-SHA1\""));
        assert!(header.contains("oauth_timestamp=\"1234567890\""));
        assert!(header.contains("oauth_nonce=\"testnonce\""));
        assert!(header.contains("oauth_version=\"1.0\""));
    }

    #[test]
    fn test_build_authorization_header_with_realm() {
        let header = build_authorization_header(
            "GET",
            "http://example.com/resource",
            &OAuth1SignatureMethod::HmacSha1,
            "ck",
            "cs",
            "at",
            "ts",
            "123",
            "nonce",
            "1.0",
            "Example",
            false,
            &[],
        );
        assert!(header.contains("realm=\"Example\""));
    }

    #[test]
    fn test_build_authorization_header_no_realm_when_empty() {
        let header = build_authorization_header(
            "GET",
            "http://example.com/resource",
            &OAuth1SignatureMethod::HmacSha1,
            "ck",
            "cs",
            "at",
            "ts",
            "123",
            "nonce",
            "1.0",
            "",
            false,
            &[],
        );
        assert!(!header.contains("realm="));
    }

    #[test]
    fn test_build_authorization_header_plaintext() {
        let header = build_authorization_header(
            "GET",
            "http://example.com/resource",
            &OAuth1SignatureMethod::Plaintext,
            "ck",
            "cs",
            "at",
            "ts",
            "123",
            "nonce",
            "1.0",
            "",
            false,
            &[],
        );
        assert!(header.contains("oauth_signature_method=\"PLAINTEXT\""));
        assert!(header.contains("oauth_signature="));
    }

    #[test]
    fn test_build_authorization_header_hmac_sha256() {
        let header = build_authorization_header(
            "POST",
            "http://example.com/resource",
            &OAuth1SignatureMethod::HmacSha256,
            "ck",
            "cs",
            "at",
            "ts",
            "123",
            "nonce",
            "1.0",
            "",
            false,
            &[],
        );
        assert!(header.contains("oauth_signature_method=\"HMAC-SHA256\""));
    }

    #[test]
    fn test_realm_not_percent_encoded() {
        let header = build_authorization_header(
            "GET",
            "http://example.com/resource",
            &OAuth1SignatureMethod::HmacSha1,
            "ck",
            "cs",
            "at",
            "ts",
            "123",
            "nonce",
            "1.0",
            "My API",
            false,
            &[],
        );
        // realm is a quoted-string per RFC 5849 Section 3.5.1 — not percent-encoded
        assert!(header.contains("realm=\"My API\""));
        assert!(!header.contains("realm=\"My%20API\""));
    }

    #[test]
    fn test_two_legged_oauth_omits_oauth_token() {
        let header = build_authorization_header(
            "GET",
            "http://example.com/resource",
            &OAuth1SignatureMethod::HmacSha1,
            "consumer_key_123",
            "consumer_secret_456",
            "",
            "",
            "1234567890",
            "testnonce",
            "1.0",
            "",
            false,
            &[],
        );
        assert!(!header.contains("oauth_token"));
    }

    #[test]
    fn test_url_normalization_removes_default_port() {
        let params = vec![("a", "1")];
        let with_port =
            signature_base_string("GET", "HTTP://PHOTOS.EXAMPLE.NET:80/photos", &params);
        let without_port =
            signature_base_string("GET", "http://photos.example.net/photos", &params);
        assert_eq!(with_port, without_port);
    }

    #[test]
    fn test_url_normalization_keeps_non_default_port() {
        let params = vec![("a", "1")];
        let with_port = signature_base_string("GET", "http://example.com:8080/path", &params);
        let without_port = signature_base_string("GET", "http://example.com/path", &params);
        assert_ne!(with_port, without_port);
    }

    #[test]
    fn test_build_authorization_header_with_extra_params() {
        // Extra params should be included in signature computation but not in the header
        let header = build_authorization_header(
            "GET",
            "http://example.com/resource",
            &OAuth1SignatureMethod::HmacSha1,
            "ck",
            "cs",
            "at",
            "ts",
            "123",
            "nonce",
            "1.0",
            "",
            false,
            &[("foo", "bar")],
        );
        // Extra params affect the signature but shouldn't appear in the header itself
        assert!(header.starts_with("OAuth "));
        assert!(!header.contains("foo="));
    }
}
