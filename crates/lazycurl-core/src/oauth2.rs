use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

use crate::types::ClientAuthentication;

/// Generate a random code verifier (43-128 characters, unreserved set).
pub fn generate_code_verifier() -> String {
    let charset = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let mut rng = rand::thread_rng();
    (0..64)
        .map(|_| charset[rng.gen_range(0..charset.len())] as char)
        .collect()
}

/// Compute S256 code challenge from verifier per RFC 7636.
pub fn code_challenge_sha256(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize())
}

/// Build the OAuth 2.0 authorization URL.
pub fn build_authorization_url(
    auth_url: &str,
    client_id: &str,
    redirect_uri: &str,
    scope: &str,
    state: &str,
    pkce: Option<(&str, &str)>, // (code_challenge, method)
) -> String {
    let mut url = Url::parse(auth_url).expect("Invalid auth URL");
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("response_type", "code");
        q.append_pair("client_id", client_id);
        q.append_pair("redirect_uri", redirect_uri);
        if !scope.is_empty() {
            q.append_pair("scope", scope);
        }
        if !state.is_empty() {
            q.append_pair("state", state);
        }
        if let Some((challenge, method)) = pkce {
            q.append_pair("code_challenge", challenge);
            q.append_pair("code_challenge_method", method);
        }
    }
    url.to_string()
}

pub struct CallbackResult {
    pub code: String,
    pub state: String,
}

/// Handle a single OAuth callback request.
pub async fn handle_callback(
    listener: TcpListener,
) -> Result<CallbackResult, Box<dyn std::error::Error + Send + Sync>> {
    let (mut stream, _) = listener.accept().await?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse the request line: "GET /callback?code=xxx&state=yyy HTTP/1.1"
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .ok_or("Invalid HTTP request")?;

    let url = Url::parse(&format!("http://localhost{}", path))?;
    let params: std::collections::HashMap<String, String> =
        url.query_pairs().into_owned().collect();

    let code = params.get("code").cloned().unwrap_or_default();
    let state = params.get("state").cloned().unwrap_or_default();

    let html =
        "<html><body><h1>Authorization complete</h1><p>You can close this tab.</p></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.len(),
        html
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;

    Ok(CallbackResult { code, state })
}

/// Build curl args for the token exchange POST request.
pub fn build_token_exchange_args(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: Option<&str>,
    client_auth: &ClientAuthentication,
) -> Vec<String> {
    let mut args = vec!["-s".to_string(), "-X".to_string(), "POST".to_string()];

    let mut body_parts = vec![
        "grant_type=authorization_code".to_string(),
        format!("code={}", code),
        format!("redirect_uri={}", redirect_uri),
    ];

    if let Some(verifier) = code_verifier {
        body_parts.push(format!("code_verifier={}", verifier));
    }

    match client_auth {
        ClientAuthentication::BasicHeader => {
            args.push("-u".to_string());
            args.push(format!("{}:{}", client_id, client_secret));
        }
        ClientAuthentication::Body => {
            body_parts.push(format!("client_id={}", client_id));
            body_parts.push(format!("client_secret={}", client_secret));
        }
    }

    args.push("-d".to_string());
    args.push(body_parts.join("&"));

    args.push("-H".to_string());
    args.push("Content-Type: application/x-www-form-urlencoded".to_string());

    args.push(token_url.to_string());
    args
}

/// Build curl args for the client credentials grant.
pub fn build_client_credentials_args(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    scope: &str,
    client_auth: &ClientAuthentication,
) -> Vec<String> {
    let mut args = vec!["-s".to_string(), "-X".to_string(), "POST".to_string()];

    let mut body_parts = vec!["grant_type=client_credentials".to_string()];

    if !scope.is_empty() {
        body_parts.push(format!("scope={}", scope));
    }

    match client_auth {
        ClientAuthentication::BasicHeader => {
            args.push("-u".to_string());
            args.push(format!("{}:{}", client_id, client_secret));
        }
        ClientAuthentication::Body => {
            body_parts.push(format!("client_id={}", client_id));
            body_parts.push(format!("client_secret={}", client_secret));
        }
    }

    args.push("-d".to_string());
    args.push(body_parts.join("&"));

    args.push("-H".to_string());
    args.push("Content-Type: application/x-www-form-urlencoded".to_string());

    args.push(token_url.to_string());
    args
}

/// Build curl args for the resource owner password credentials grant.
pub fn build_password_grant_args(
    token_url: &str,
    username: &str,
    password: &str,
    client_id: &str,
    client_secret: &str,
    scope: &str,
    client_auth: &ClientAuthentication,
) -> Vec<String> {
    let mut args = vec!["-s".to_string(), "-X".to_string(), "POST".to_string()];

    let mut body_parts = vec![
        "grant_type=password".to_string(),
        format!("username={}", username),
        format!("password={}", password),
    ];

    if !scope.is_empty() {
        body_parts.push(format!("scope={}", scope));
    }

    match client_auth {
        ClientAuthentication::BasicHeader => {
            args.push("-u".to_string());
            args.push(format!("{}:{}", client_id, client_secret));
        }
        ClientAuthentication::Body => {
            body_parts.push(format!("client_id={}", client_id));
            body_parts.push(format!("client_secret={}", client_secret));
        }
    }

    args.push("-d".to_string());
    args.push(body_parts.join("&"));

    args.push("-H".to_string());
    args.push("Content-Type: application/x-www-form-urlencoded".to_string());

    args.push(token_url.to_string());
    args
}

/// Build curl args for the refresh token grant.
pub fn build_refresh_token_args(
    token_url: &str,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
    client_auth: &ClientAuthentication,
) -> Vec<String> {
    let mut args = vec!["-s".to_string(), "-X".to_string(), "POST".to_string()];

    let mut body_parts = vec![
        "grant_type=refresh_token".to_string(),
        format!("refresh_token={}", refresh_token),
    ];

    match client_auth {
        ClientAuthentication::BasicHeader => {
            args.push("-u".to_string());
            args.push(format!("{}:{}", client_id, client_secret));
        }
        ClientAuthentication::Body => {
            body_parts.push(format!("client_id={}", client_id));
            body_parts.push(format!("client_secret={}", client_secret));
        }
    }

    args.push("-d".to_string());
    args.push(body_parts.join("&"));

    args.push("-H".to_string());
    args.push("Content-Type: application/x-www-form-urlencoded".to_string());

    args.push(token_url.to_string());
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code_verifier() {
        let verifier = generate_code_verifier();
        assert!(verifier.len() >= 43 && verifier.len() <= 128);
        // Must only contain unreserved characters
        assert!(verifier.chars().all(|c| c.is_ascii_alphanumeric()
            || c == '-'
            || c == '.'
            || c == '_'
            || c == '~'));
    }

    #[test]
    fn test_code_challenge_sha256() {
        // RFC 7636 Appendix B test vector
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = code_challenge_sha256(verifier);
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn test_build_auth_url_authorization_code() {
        let url = build_authorization_url(
            "https://auth.example.com/authorize",
            "my-client-id",
            "http://localhost:9876/callback",
            "read write",
            "state123",
            None, // No PKCE
        );
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=my-client-id"));
        assert!(url.contains("redirect_uri=http"));
        assert!(url.contains("scope=read+write") || url.contains("scope=read%20write"));
        assert!(url.contains("state=state123"));
    }

    #[test]
    fn test_build_auth_url_pkce() {
        let url = build_authorization_url(
            "https://auth.example.com/authorize",
            "my-client-id",
            "http://localhost:9876/callback",
            "read",
            "state123",
            Some(("challenge_value", "S256")),
        );
        assert!(url.contains("code_challenge=challenge_value"));
        assert!(url.contains("code_challenge_method=S256"));
    }

    #[tokio::test]
    async fn test_callback_server_captures_code() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // Spawn callback server
        tokio::spawn(async move {
            let result = handle_callback(listener).await;
            tx.send(result).ok();
        });

        // Simulate provider redirect
        let client = reqwest::Client::new();
        let resp = client
            .get(format!(
                "http://127.0.0.1:{}/callback?code=test_code_123&state=test_state",
                port
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        let result = rx.await.unwrap().unwrap();
        assert_eq!(result.code, "test_code_123");
        assert_eq!(result.state, "test_state");
    }

    #[test]
    fn test_build_token_exchange_args_auth_code() {
        let args = build_token_exchange_args(
            "https://auth.example.com/token",
            "my-client-id",
            "my-secret",
            "auth_code_123",
            "http://localhost:9876/callback",
            None, // no PKCE verifier
            &crate::types::ClientAuthentication::BasicHeader,
        );
        assert!(args.contains(&"-X".to_string()));
        assert!(args.contains(&"POST".to_string()));
        assert!(args.contains(&"https://auth.example.com/token".to_string()));
        // Should contain grant_type=authorization_code in body
        let body_idx = args.iter().position(|a| a == "-d").unwrap();
        let body = &args[body_idx + 1];
        assert!(body.contains("grant_type=authorization_code"));
        assert!(body.contains("code=auth_code_123"));
        // Basic header auth: -u client_id:secret
        assert!(args.contains(&"-u".to_string()));
    }

    #[test]
    fn test_build_token_exchange_args_with_pkce() {
        let args = build_token_exchange_args(
            "https://auth.example.com/token",
            "my-client-id",
            "my-secret",
            "auth_code_123",
            "http://localhost:9876/callback",
            Some("my_verifier"),
            &ClientAuthentication::BasicHeader,
        );
        let body_idx = args.iter().position(|a| a == "-d").unwrap();
        let body = &args[body_idx + 1];
        assert!(body.contains("code_verifier=my_verifier"));
    }

    #[test]
    fn test_build_token_exchange_args_body_auth() {
        let args = build_token_exchange_args(
            "https://auth.example.com/token",
            "my-client-id",
            "my-secret",
            "auth_code_123",
            "http://localhost:9876/callback",
            None,
            &ClientAuthentication::Body,
        );
        let body_idx = args.iter().position(|a| a == "-d").unwrap();
        let body = &args[body_idx + 1];
        assert!(body.contains("client_id=my-client-id"));
        assert!(body.contains("client_secret=my-secret"));
        assert!(!args.contains(&"-u".to_string()));
    }

    #[test]
    fn test_build_client_credentials_args_basic() {
        let args = build_client_credentials_args(
            "https://auth.example.com/token",
            "my-client-id",
            "my-secret",
            "read write",
            &ClientAuthentication::BasicHeader,
        );
        assert!(args.contains(&"-X".to_string()));
        assert!(args.contains(&"POST".to_string()));
        let body_idx = args.iter().position(|a| a == "-d").unwrap();
        let body = &args[body_idx + 1];
        assert!(body.contains("grant_type=client_credentials"));
        assert!(body.contains("scope=read+write") || body.contains("scope=read write"));
        assert!(args.contains(&"-u".to_string()));
    }

    #[test]
    fn test_build_client_credentials_args_body() {
        let args = build_client_credentials_args(
            "https://auth.example.com/token",
            "my-client-id",
            "my-secret",
            "",
            &ClientAuthentication::Body,
        );
        let body_idx = args.iter().position(|a| a == "-d").unwrap();
        let body = &args[body_idx + 1];
        assert!(body.contains("grant_type=client_credentials"));
        assert!(body.contains("client_id=my-client-id"));
        assert!(body.contains("client_secret=my-secret"));
        // No scope when empty
        assert!(!body.contains("scope="));
    }

    #[test]
    fn test_build_password_grant_args() {
        let args = build_password_grant_args(
            "https://auth.example.com/token",
            "user@example.com",
            "p@ssw0rd",
            "my-client-id",
            "my-secret",
            "read",
            &ClientAuthentication::BasicHeader,
        );
        assert!(args.contains(&"POST".to_string()));
        let body_idx = args.iter().position(|a| a == "-d").unwrap();
        let body = &args[body_idx + 1];
        assert!(body.contains("grant_type=password"));
        assert!(body.contains("username=user@example.com"));
        assert!(body.contains("password=p@ssw0rd"));
        assert!(body.contains("scope=read"));
        assert!(args.contains(&"-u".to_string()));
    }

    #[test]
    fn test_build_refresh_token_args() {
        let args = build_refresh_token_args(
            "https://auth.example.com/token",
            "my_refresh_token",
            "my-client-id",
            "my-secret",
            &ClientAuthentication::BasicHeader,
        );
        assert!(args.contains(&"POST".to_string()));
        let body_idx = args.iter().position(|a| a == "-d").unwrap();
        let body = &args[body_idx + 1];
        assert!(body.contains("grant_type=refresh_token"));
        assert!(body.contains("refresh_token=my_refresh_token"));
        assert!(args.contains(&"-u".to_string()));
    }

    #[test]
    fn test_build_refresh_token_args_body_auth() {
        let args = build_refresh_token_args(
            "https://auth.example.com/token",
            "my_refresh_token",
            "my-client-id",
            "my-secret",
            &ClientAuthentication::Body,
        );
        let body_idx = args.iter().position(|a| a == "-d").unwrap();
        let body = &args[body_idx + 1];
        assert!(body.contains("client_id=my-client-id"));
        assert!(body.contains("client_secret=my-secret"));
    }
}
