use crate::types::AsapAlgorithm;
use base64::Engine;
use p256::ecdsa::SigningKey as P256SigningKey;
use p384::ecdsa::SigningKey as P384SigningKey;
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::signature::{SignatureEncoding, Signer};
use sha2::{Sha256, Sha384, Sha512};

/// URL-safe base64 encoding without padding.
pub fn base64url_encode(input: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(input)
}

/// Build JWT header and claims JSON strings.
///
/// Returns `(header_json, claims_json)`.
///
/// - `iat_override`: if > 0, use as `iat`; otherwise use current time.
/// - `jti_override`: if non-empty, use as `jti`; otherwise generate a UUID.
/// - `subject`: if empty, defaults to `issuer`.
/// - `additional_claims`: if non-empty valid JSON object, merge keys into claims.
#[allow(clippy::too_many_arguments)]
pub fn build_jwt_parts(
    algorithm: &AsapAlgorithm,
    issuer: &str,
    audience: &str,
    key_id: &str,
    subject: &str,
    expiry_seconds: u64,
    additional_claims: &str,
    iat_override: u64,
    jti_override: &str,
) -> (String, String) {
    let alg_str = match algorithm {
        AsapAlgorithm::RS256 => "RS256",
        AsapAlgorithm::RS384 => "RS384",
        AsapAlgorithm::RS512 => "RS512",
        AsapAlgorithm::PS256 => "PS256",
        AsapAlgorithm::PS384 => "PS384",
        AsapAlgorithm::PS512 => "PS512",
        AsapAlgorithm::ES256 => "ES256",
        AsapAlgorithm::ES384 => "ES384",
        AsapAlgorithm::ES512 => "ES512",
    };

    let header = serde_json::json!({
        "alg": alg_str,
        "kid": key_id,
        "typ": "JWT",
    });

    let iat = if iat_override > 0 {
        iat_override
    } else {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    };

    let exp = iat + expiry_seconds;

    let jti = if jti_override.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        jti_override.to_string()
    };

    let sub = if subject.is_empty() {
        issuer.to_string()
    } else {
        subject.to_string()
    };

    let mut claims = serde_json::json!({
        "iss": issuer,
        "aud": audience,
        "iat": iat,
        "exp": exp,
        "nbf": iat,
        "jti": jti,
        "sub": sub,
    });

    if !additional_claims.is_empty() {
        if let Ok(extra) = serde_json::from_str::<serde_json::Value>(additional_claims) {
            if let Some(obj) = extra.as_object() {
                for (k, v) in obj {
                    claims[k] = v.clone();
                }
            }
        }
    }

    (header.to_string(), claims.to_string())
}

/// Encode JWT header and claims as `base64url(header).base64url(claims)`.
pub fn encode_jwt_unsigned(header: &str, claims: &str) -> String {
    let h = base64url_encode(header.as_bytes());
    let c = base64url_encode(claims.as_bytes());
    format!("{h}.{c}")
}

/// Build and sign a complete JWT: `header.claims.signature`.
#[allow(clippy::too_many_arguments)]
pub fn build_and_sign_jwt(
    algorithm: &AsapAlgorithm,
    issuer: &str,
    audience: &str,
    key_id: &str,
    subject: &str,
    expiry_seconds: u64,
    additional_claims: &str,
    private_key_pem: &str,
    iat_override: u64,
    jti_override: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let (header, claims) = build_jwt_parts(
        algorithm,
        issuer,
        audience,
        key_id,
        subject,
        expiry_seconds,
        additional_claims,
        iat_override,
        jti_override,
    );

    let unsigned = encode_jwt_unsigned(&header, &claims);

    let signature_bytes: Vec<u8> = match algorithm {
        AsapAlgorithm::RS256 => {
            let key = rsa::RsaPrivateKey::from_pkcs8_pem(private_key_pem)?;
            let signing_key = SigningKey::<Sha256>::new(key);
            signing_key.sign(unsigned.as_bytes()).to_bytes().to_vec()
        }
        AsapAlgorithm::RS384 => {
            let key = rsa::RsaPrivateKey::from_pkcs8_pem(private_key_pem)?;
            let signing_key = SigningKey::<Sha384>::new(key);
            signing_key.sign(unsigned.as_bytes()).to_bytes().to_vec()
        }
        AsapAlgorithm::RS512 => {
            let key = rsa::RsaPrivateKey::from_pkcs8_pem(private_key_pem)?;
            let signing_key = SigningKey::<Sha512>::new(key);
            signing_key.sign(unsigned.as_bytes()).to_bytes().to_vec()
        }
        AsapAlgorithm::ES256 => {
            let signing_key = P256SigningKey::from_pkcs8_pem(private_key_pem)?;
            let sig: p256::ecdsa::Signature =
                p256::ecdsa::signature::Signer::sign(&signing_key, unsigned.as_bytes());
            sig.to_bytes().to_vec()
        }
        AsapAlgorithm::ES384 => {
            let signing_key = P384SigningKey::from_pkcs8_pem(private_key_pem)?;
            let sig: p384::ecdsa::Signature =
                p384::ecdsa::signature::Signer::sign(&signing_key, unsigned.as_bytes());
            sig.to_bytes().to_vec()
        }
        AsapAlgorithm::PS256
        | AsapAlgorithm::PS384
        | AsapAlgorithm::PS512
        | AsapAlgorithm::ES512 => {
            return Err("Algorithm not yet supported".into());
        }
    };

    let sig_encoded = base64url_encode(&signature_bytes);
    Ok(format!("{unsigned}.{sig_encoded}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::pkcs8::EncodePrivateKey;

    #[test]
    fn test_base64url_encode() {
        let input = b"Hello, World!";
        let encoded = base64url_encode(input);
        // No padding characters
        assert!(!encoded.contains('='));
        // No standard base64 chars that are not URL-safe
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
        // Known value
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ");
    }

    #[test]
    fn test_build_jwt_parts() {
        let (header, claims) = build_jwt_parts(
            &AsapAlgorithm::RS256,
            "my-issuer",
            "my-audience",
            "my-key-id",
            "",
            3600,
            "",
            1700000000,
            "test-jti-123",
        );

        let h: serde_json::Value = serde_json::from_str(&header).unwrap();
        assert_eq!(h["alg"], "RS256");
        assert_eq!(h["kid"], "my-key-id");
        assert_eq!(h["typ"], "JWT");

        let c: serde_json::Value = serde_json::from_str(&claims).unwrap();
        assert_eq!(c["iss"], "my-issuer");
        assert_eq!(c["aud"], "my-audience");
        assert_eq!(c["iat"], 1700000000);
        assert_eq!(c["exp"], 1700003600);
        assert_eq!(c["nbf"], 1700000000);
        assert_eq!(c["jti"], "test-jti-123");
        // Subject defaults to issuer when empty
        assert_eq!(c["sub"], "my-issuer");
    }

    #[test]
    fn test_build_jwt_parts_with_additional_claims() {
        let extra = r#"{"custom_claim":"value1","priority":42}"#;
        let (_header, claims) = build_jwt_parts(
            &AsapAlgorithm::ES256,
            "issuer",
            "audience",
            "kid",
            "explicit-subject",
            60,
            extra,
            1700000000,
            "jti-abc",
        );

        let c: serde_json::Value = serde_json::from_str(&claims).unwrap();
        assert_eq!(c["custom_claim"], "value1");
        assert_eq!(c["priority"], 42);
        assert_eq!(c["sub"], "explicit-subject");
    }

    #[test]
    fn test_encode_jwt_unsigned() {
        let header = r#"{"alg":"RS256","kid":"k1","typ":"JWT"}"#;
        let claims = r#"{"iss":"i","aud":"a","iat":0,"exp":60}"#;
        let result = encode_jwt_unsigned(header, claims);

        // Exactly one dot separating two parts
        let parts: Vec<&str> = result.split('.').collect();
        assert_eq!(parts.len(), 2);

        // No padding
        assert!(!result.contains('='));
    }

    #[test]
    fn test_sign_jwt_rs256() {
        let mut rng = rand::thread_rng();
        let private_key = rsa::RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let pem = private_key
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .unwrap();

        let jwt = build_and_sign_jwt(
            &AsapAlgorithm::RS256,
            "test-issuer",
            "test-audience",
            "key-1",
            "",
            3600,
            "",
            pem.as_ref(),
            1700000000,
            "jti-rs256",
        )
        .unwrap();

        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        // All parts should be non-empty
        assert!(parts.iter().all(|p| !p.is_empty()));
        // No padding in any part
        assert!(!jwt.contains('='));
    }

    #[test]
    fn test_sign_jwt_es256() {
        let mut rng = rand::thread_rng();
        let secret_key = p256::SecretKey::random(&mut rng);
        let pem = secret_key
            .to_pkcs8_pem(p256::pkcs8::LineEnding::LF)
            .unwrap();

        let jwt = build_and_sign_jwt(
            &AsapAlgorithm::ES256,
            "test-issuer",
            "test-audience",
            "key-ec",
            "subject",
            60,
            "",
            pem.as_ref(),
            1700000000,
            "jti-es256",
        )
        .unwrap();

        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|p| !p.is_empty()));
    }

    #[test]
    fn test_unsupported_algorithm() {
        let result = build_and_sign_jwt(
            &AsapAlgorithm::PS256,
            "issuer",
            "audience",
            "kid",
            "",
            60,
            "",
            "not-needed",
            1700000000,
            "jti",
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Algorithm not yet supported"
        );
    }
}
