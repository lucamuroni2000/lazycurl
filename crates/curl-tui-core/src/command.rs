use crate::secret;
use crate::types::{CurlResponse, Method, ResponseTiming};

/// Returns the platform-appropriate curl binary name.
pub fn curl_binary() -> &'static str {
    if cfg!(windows) {
        "curl.exe"
    } else {
        "curl"
    }
}

/// A built curl command, ready to execute or display.
pub struct CurlCommand {
    url: String,
    method: Option<Method>,
    headers: Vec<(String, String)>,
    body: Option<String>,
    form_fields: Vec<(String, String)>,
    multipart_fields: Vec<(String, String)>,
    multipart_files: Vec<(String, String)>,
    timeout: Option<u32>,
    basic_auth: Option<(String, String)>,
    cookies: Vec<String>,
    follow_redirects: bool,
    query_params: Vec<(String, String)>,
}

impl CurlCommand {
    /// Build the curl argument vector (without the `curl` binary itself).
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Always silent
        args.push("-s".to_string());

        // Method (skip for GET as it's the default)
        if let Some(method) = &self.method {
            if !matches!(method, Method::Get) {
                args.push("-X".to_string());
                args.push(method.to_string());
            }
        }

        // Headers
        for (key, value) in &self.headers {
            args.push("-H".to_string());
            args.push(format!("{}: {}", key, value));
        }

        // Body
        if let Some(body) = &self.body {
            args.push("-d".to_string());
            args.push(body.clone());
        }

        // Form fields (url-encoded)
        for (key, value) in &self.form_fields {
            args.push("--data-urlencode".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Multipart fields
        for (key, value) in &self.multipart_fields {
            args.push("-F".to_string());
            args.push(format!("{}={}", key, value));
        }

        // Multipart files
        for (key, path) in &self.multipart_files {
            args.push("-F".to_string());
            args.push(format!("{}=@{}", key, path));
        }

        // Timeout
        if let Some(t) = self.timeout {
            args.push("--max-time".to_string());
            args.push(t.to_string());
        }

        // Basic auth
        if let Some((user, pass)) = &self.basic_auth {
            args.push("-u".to_string());
            args.push(format!("{}:{}", user, pass));
        }

        // Cookies
        for cookie in &self.cookies {
            args.push("-b".to_string());
            args.push(cookie.clone());
        }

        // Follow redirects
        if self.follow_redirects {
            args.push("-L".to_string());
        }

        // URL with query params
        let url = self.build_url();
        args.push(url);

        args
    }

    fn build_url(&self) -> String {
        if self.query_params.is_empty() {
            return self.url.clone();
        }
        let params: Vec<String> = self
            .query_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let separator = if self.url.contains('?') { "&" } else { "?" };
        format!("{}{}{}", self.url, separator, params.join("&"))
    }

    /// Render the command as a display string, redacting secret values.
    pub fn to_display_string(&self, secrets: &[String]) -> String {
        let args = self.to_args();
        let mut parts = vec![curl_binary().to_string()];
        parts.extend(args);
        let full = parts.join(" ");
        secret::redact_secrets(&full, secrets)
    }

    /// Execute the curl command as a subprocess.
    pub async fn execute(&self) -> Result<CurlResponse, Box<dyn std::error::Error>> {
        let mut args = self.to_args();

        // Create temp files for body and headers
        let body_file = tempfile::NamedTempFile::new()?;
        let header_file = tempfile::NamedTempFile::new()?;

        // Remove the URL (last arg) and re-add with output options
        let url = args.pop().unwrap();
        args.push("-o".to_string());
        args.push(body_file.path().to_string_lossy().to_string());
        args.push("-D".to_string());
        args.push(header_file.path().to_string_lossy().to_string());
        args.push("-w".to_string());
        args.push("%{json}".to_string());
        args.push(url);

        let output = tokio::process::Command::new(curl_binary())
            .args(&args)
            .output()
            .await?;

        let write_out = String::from_utf8_lossy(&output.stdout).to_string();
        let body = std::fs::read_to_string(body_file.path()).unwrap_or_default();
        let raw_headers = std::fs::read_to_string(header_file.path()).unwrap_or_default();
        let headers = parse_headers(&raw_headers);

        // Parse write-out JSON for status code and timing
        let (status_code, timing) = parse_write_out(&write_out);

        Ok(CurlResponse {
            status_code,
            headers,
            body,
            timing,
            raw_command: self.to_display_string(&[]),
        })
    }
}

/// Parse raw HTTP headers from curl's -D output.
pub fn parse_headers(raw: &str) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.starts_with("HTTP/") || line.is_empty() {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.push((key.trim().to_string(), value.trim().to_string()));
        }
    }
    headers
}

/// Parse curl's -w %{json} write-out.
fn parse_write_out(json_str: &str) -> (u16, ResponseTiming) {
    let default_timing = ResponseTiming {
        dns_lookup_ms: 0.0,
        tcp_connect_ms: 0.0,
        tls_handshake_ms: 0.0,
        transfer_start_ms: 0.0,
        total_ms: 0.0,
    };

    let parsed: serde_json::Value = match serde_json::from_str(json_str.trim()) {
        Ok(v) => v,
        Err(_) => return (0, default_timing),
    };

    let status_code = parsed
        .get("http_code")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u16;

    let get_ms = |key: &str| -> f64 {
        parsed
            .get(key)
            .and_then(|v| v.as_f64())
            .map(|s| s * 1000.0) // curl reports seconds, we want ms
            .unwrap_or(0.0)
    };

    let timing = ResponseTiming {
        dns_lookup_ms: get_ms("time_namelookup"),
        tcp_connect_ms: get_ms("time_connect"),
        tls_handshake_ms: get_ms("time_appconnect"),
        transfer_start_ms: get_ms("time_starttransfer"),
        total_ms: get_ms("time_total"),
    };

    (status_code, timing)
}

pub struct CurlCommandBuilder {
    url: String,
    method: Option<Method>,
    headers: Vec<(String, String)>,
    body: Option<String>,
    form_fields: Vec<(String, String)>,
    multipart_fields: Vec<(String, String)>,
    multipart_files: Vec<(String, String)>,
    timeout: Option<u32>,
    basic_auth: Option<(String, String)>,
    cookies: Vec<String>,
    follow_redirects: bool,
    query_params: Vec<(String, String)>,
}

impl CurlCommandBuilder {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            method: None,
            headers: Vec::new(),
            body: None,
            form_fields: Vec::new(),
            multipart_fields: Vec::new(),
            multipart_files: Vec::new(),
            timeout: None,
            basic_auth: None,
            cookies: Vec::new(),
            follow_redirects: false,
            query_params: Vec::new(),
        }
    }

    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_string(), value.to_string()));
        self
    }

    pub fn body_json(mut self, json: &str) -> Self {
        self.body = Some(json.to_string());
        self
    }

    pub fn body_text(mut self, text: &str) -> Self {
        self.body = Some(text.to_string());
        self
    }

    pub fn form_field(mut self, key: &str, value: &str) -> Self {
        self.form_fields.push((key.to_string(), value.to_string()));
        self
    }

    pub fn multipart_field(mut self, key: &str, value: &str) -> Self {
        self.multipart_fields
            .push((key.to_string(), value.to_string()));
        self
    }

    pub fn multipart_file(mut self, key: &str, path: &str) -> Self {
        self.multipart_files
            .push((key.to_string(), path.to_string()));
        self
    }

    pub fn timeout(mut self, seconds: u32) -> Self {
        self.timeout = Some(seconds);
        self
    }

    pub fn basic_auth(mut self, user: &str, pass: &str) -> Self {
        self.basic_auth = Some((user.to_string(), pass.to_string()));
        self
    }

    pub fn cookie(mut self, cookie: &str) -> Self {
        self.cookies.push(cookie.to_string());
        self
    }

    pub fn follow_redirects(mut self, follow: bool) -> Self {
        self.follow_redirects = follow;
        self
    }

    pub fn query_param(mut self, key: &str, value: &str) -> Self {
        self.query_params.push((key.to_string(), value.to_string()));
        self
    }

    pub fn build(self) -> CurlCommand {
        CurlCommand {
            url: self.url,
            method: self.method,
            headers: self.headers,
            body: self.body,
            form_fields: self.form_fields,
            multipart_fields: self.multipart_fields,
            multipart_files: self.multipart_files,
            timeout: self.timeout,
            basic_auth: self.basic_auth,
            cookies: self.cookies,
            follow_redirects: self.follow_redirects,
            query_params: self.query_params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Method;

    #[test]
    fn test_basic_get_args() {
        let cmd = CurlCommandBuilder::new("https://api.example.com/users").build();
        let args = cmd.to_args();
        assert!(args.contains(&"-s".to_string()));
        assert!(args.contains(&"https://api.example.com/users".to_string()));
        // GET is default, no -X needed
        assert!(!args.contains(&"-X".to_string()));
    }

    #[test]
    fn test_post_method() {
        let cmd = CurlCommandBuilder::new("https://api.example.com/users")
            .method(Method::Post)
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"-X".to_string()));
        assert!(args.contains(&"POST".to_string()));
    }

    #[test]
    fn test_headers() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .build();
        let args = cmd.to_args();
        let h_positions: Vec<usize> = args
            .iter()
            .enumerate()
            .filter(|(_, a)| *a == "-H")
            .map(|(i, _)| i)
            .collect();
        assert_eq!(h_positions.len(), 2);
        assert_eq!(args[h_positions[0] + 1], "Content-Type: application/json");
        assert_eq!(args[h_positions[1] + 1], "Accept: application/json");
    }

    #[test]
    fn test_json_body() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .method(Method::Post)
            .body_json(r#"{"name": "Alice"}"#)
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"-d".to_string()));
        assert!(args.contains(&r#"{"name": "Alice"}"#.to_string()));
    }

    #[test]
    fn test_timeout() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .timeout(30)
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"--max-time".to_string()));
        assert!(args.contains(&"30".to_string()));
    }

    #[test]
    fn test_basic_auth() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .basic_auth("user", "pass")
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"-u".to_string()));
        assert!(args.contains(&"user:pass".to_string()));
    }

    #[test]
    fn test_cookie() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .cookie("session=abc123")
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"-b".to_string()));
        assert!(args.contains(&"session=abc123".to_string()));
    }

    #[test]
    fn test_follow_redirects() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .follow_redirects(true)
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"-L".to_string()));
    }

    #[test]
    fn test_display_string_basic() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .method(Method::Get)
            .build();
        let display = cmd.to_display_string(&[]);
        assert!(display.starts_with("curl"));
        assert!(display.contains("https://example.com"));
    }

    #[test]
    fn test_display_string_redacts_secrets() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .header("Authorization", "Bearer secret-token-123")
            .build();
        let display = cmd.to_display_string(&["secret-token-123".to_string()]);
        assert!(!display.contains("secret-token-123"));
        assert!(display.contains("••••••"));
    }

    #[test]
    fn test_query_params() {
        let cmd = CurlCommandBuilder::new("https://example.com/api")
            .query_param("page", "1")
            .query_param("limit", "20")
            .build();
        let args = cmd.to_args();
        // URL should have query params appended
        let url = args.last().unwrap();
        assert!(url.contains("page=1"));
        assert!(url.contains("limit=20"));
        assert!(url.contains('?'));
        assert!(url.contains('&'));
    }

    #[test]
    fn test_form_data() {
        let cmd = CurlCommandBuilder::new("https://example.com")
            .method(Method::Post)
            .form_field("username", "alice")
            .form_field("password", "s3cret")
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"--data-urlencode".to_string()));
        assert!(args.contains(&"username=alice".to_string()));
    }

    #[test]
    fn test_multipart_upload() {
        let cmd = CurlCommandBuilder::new("https://example.com/upload")
            .method(Method::Post)
            .multipart_field("description", "My file")
            .multipart_file("file", "/path/to/file.png")
            .build();
        let args = cmd.to_args();
        assert!(args.contains(&"-F".to_string()));
        assert!(args.contains(&"description=My file".to_string()));
        assert!(args.contains(&"file=@/path/to/file.png".to_string()));
    }

    #[test]
    fn test_parse_response_headers() {
        let raw_headers =
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nX-Custom: value\r\n\r\n";
        let headers = parse_headers(raw_headers);
        assert_eq!(headers.len(), 2);
        assert_eq!(
            headers[0],
            ("Content-Type".to_string(), "application/json".to_string())
        );
        assert_eq!(headers[1], ("X-Custom".to_string(), "value".to_string()));
    }

    #[test]
    fn test_curl_binary_name() {
        let name = curl_binary();
        if cfg!(windows) {
            assert_eq!(name, "curl.exe");
        } else {
            assert_eq!(name, "curl");
        }
    }
}
