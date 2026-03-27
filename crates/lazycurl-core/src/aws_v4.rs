use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

/// Compute SHA-256 hex digest of the input string.
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex_encode(&hasher.finalize())
}

/// Build the AWS canonical request string.
///
/// Format:
/// ```text
/// METHOD\nURI\nQUERY\ncanonical_headers\n\nsigned_headers\npayload_hash
/// ```
///
/// Headers are sorted by lowercase key, formatted as `key:trimmed_value`.
/// Signed headers are the sorted lowercase keys joined by `;`.
pub fn canonical_request(
    method: &str,
    uri: &str,
    query_string: &str,
    headers: &[(&str, &str)],
    payload: &str,
) -> String {
    // Sort headers by lowercase key
    let mut sorted: Vec<(String, String)> = headers
        .iter()
        .map(|(k, v)| (k.to_lowercase(), v.trim().to_string()))
        .collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    let canonical_headers: String = sorted
        .iter()
        .map(|(k, v)| format!("{}:{}", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    let signed_headers: String = sorted
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<_>>()
        .join(";");

    let payload_hash = sha256_hex(payload);

    format!(
        "{}\n{}\n{}\n{}\n\n{}\n{}",
        method, uri, query_string, canonical_headers, signed_headers, payload_hash
    )
}

/// Derive the signing key via HMAC chain:
/// `AWS4{secret}` -> date -> region -> service -> `aws4_request`
pub fn signing_key(secret_key: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_secret = format!("AWS4{}", secret_key);
    let k_date = hmac_sha256(k_secret.as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

/// Build the string to sign.
///
/// Format:
/// ```text
/// AWS4-HMAC-SHA256\n{datetime}\n{date}/{region}/{service}/aws4_request\n{hash_of_canonical_request}
/// ```
pub fn string_to_sign(
    datetime: &str,
    date: &str,
    region: &str,
    service: &str,
    canonical_request: &str,
) -> String {
    let cr_hash = sha256_hex(canonical_request);
    format!(
        "AWS4-HMAC-SHA256\n{}\n{}/{}/{}/aws4_request\n{}",
        datetime, date, region, service, cr_hash
    )
}

/// Orchestrate the full AWS Signature V4 signing flow and return the Authorization header value.
///
/// The `session_token` parameter is accepted but not used directly here; the caller is
/// responsible for adding the `X-Amz-Security-Token` header to the request.
#[allow(clippy::too_many_arguments)]
pub fn build_authorization_header(
    method: &str,
    uri: &str,
    query_string: &str,
    headers: &[(&str, &str)],
    payload: &str,
    access_key: &str,
    secret_key: &str,
    region: &str,
    service: &str,
    datetime: &str,
    _session_token: Option<&str>,
) -> String {
    let date = &datetime[..8];

    let creq = canonical_request(method, uri, query_string, headers, payload);
    let sts = string_to_sign(datetime, date, region, service, &creq);
    let key = signing_key(secret_key, date, region, service);
    let signature = hex_encode(&hmac_sha256(&key, sts.as_bytes()));

    // Compute signed headers (same logic as canonical_request)
    let mut sorted_keys: Vec<String> = headers.iter().map(|(k, _)| k.to_lowercase()).collect();
    sorted_keys.sort();
    let signed_headers = sorted_keys.join(";");

    format!(
        "AWS4-HMAC-SHA256 Credential={}/{}/{}/{}/aws4_request, SignedHeaders={}, Signature={}",
        access_key, date, region, service, signed_headers, signature
    )
}

// -- internal helpers --

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hex_empty_string() {
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sha256_hex_non_empty() {
        // Known SHA-256 of "hello"
        assert_eq!(
            sha256_hex("hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_canonical_request_get_empty_query() {
        let headers = [
            ("host", "example.amazonaws.com"),
            ("x-amz-date", "20150830T123600Z"),
        ];
        let result = canonical_request("GET", "/", "", &headers, "");
        let expected = "GET\n/\n\nhost:example.amazonaws.com\nx-amz-date:20150830T123600Z\n\nhost;x-amz-date\ne3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_canonical_request_with_query_string() {
        let headers = [("host", "example.amazonaws.com")];
        let result = canonical_request(
            "GET",
            "/",
            "Action=ListUsers&Version=2010-05-08",
            &headers,
            "",
        );
        assert!(result.contains("Action=ListUsers&Version=2010-05-08"));
    }

    #[test]
    fn test_canonical_request_multiple_headers_sorted() {
        let headers = [
            ("x-amz-date", "20150830T123600Z"),
            ("content-type", "application/json"),
            ("host", "example.amazonaws.com"),
        ];
        let result = canonical_request("POST", "/", "", &headers, "{}");
        // Headers must be sorted: content-type, host, x-amz-date
        let lines: Vec<&str> = result.lines().collect();
        // Line 0: method, 1: uri, 2: query, 3-5: headers, 6: empty (separator), 7: signed_headers, 8: payload hash
        assert_eq!(lines[3], "content-type:application/json");
        assert_eq!(lines[4], "host:example.amazonaws.com");
        assert_eq!(lines[5], "x-amz-date:20150830T123600Z");
        assert!(result.contains("content-type;host;x-amz-date"));
    }

    #[test]
    fn test_signing_key_length() {
        let key = signing_key(
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "20150830",
            "us-east-1",
            "iam",
        );
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_signing_key_deterministic() {
        let key1 = signing_key(
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "20150830",
            "us-east-1",
            "iam",
        );
        let key2 = signing_key(
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "20150830",
            "us-east-1",
            "iam",
        );
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_string_to_sign_format() {
        let creq = "GET\n/\n\nhost:example.amazonaws.com\nx-amz-date:20150830T123600Z\n\nhost;x-amz-date\ne3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let result = string_to_sign("20150830T123600Z", "20150830", "us-east-1", "iam", creq);
        assert!(result.starts_with("AWS4-HMAC-SHA256\n"));
        assert!(result.contains("20150830T123600Z"));
        assert!(result.contains("20150830/us-east-1/iam/aws4_request"));
    }

    #[test]
    fn test_build_authorization_header_format() {
        let headers = [
            ("host", "example.amazonaws.com"),
            ("x-amz-date", "20150830T123600Z"),
        ];
        let result = build_authorization_header(
            "GET",
            "/",
            "",
            &headers,
            "",
            "AKIDEXAMPLE",
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "us-east-1",
            "service",
            "20150830T123600Z",
            None,
        );
        assert!(result.starts_with(
            "AWS4-HMAC-SHA256 Credential=AKIDEXAMPLE/20150830/us-east-1/service/aws4_request"
        ));
        assert!(result.contains("SignedHeaders=host;x-amz-date"));
        assert!(result.contains("Signature="));
        // Signature should be a hex string (64 hex chars for SHA-256)
        let sig = result.split("Signature=").nth(1).unwrap();
        assert_eq!(sig.len(), 64);
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_build_authorization_header_with_session_token() {
        let headers = [
            ("host", "example.amazonaws.com"),
            ("x-amz-date", "20150830T123600Z"),
        ];
        // session_token is accepted but passed through; caller adds the header
        let result = build_authorization_header(
            "GET",
            "/",
            "",
            &headers,
            "",
            "AKIDEXAMPLE",
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "us-east-1",
            "service",
            "20150830T123600Z",
            Some("my-session-token"),
        );
        assert!(result.starts_with("AWS4-HMAC-SHA256 Credential="));
    }
}
