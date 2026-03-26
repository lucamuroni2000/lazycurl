/// The redaction placeholder shown in place of secret values.
pub const REDACTED_DISPLAY: &str = "••••••";

/// The redaction placeholder used in persisted output (history, logs).
pub const REDACTED_LOG: &str = "[REDACTED]";

/// Replace any value with the display redaction placeholder.
pub fn redact(_value: &str) -> &'static str {
    REDACTED_DISPLAY
}

/// Replace all occurrences of `secret` within `input` with the redaction placeholder.
pub fn redact_in_string(input: &str, secret: &str) -> String {
    input.replace(secret, REDACTED_DISPLAY)
}

/// Replace all known secret values in `text`.
pub fn redact_secrets(text: &str, secrets: &[String]) -> String {
    let mut result = text.to_string();
    for secret in secrets {
        if !secret.is_empty() {
            result = result.replace(secret.as_str(), REDACTED_DISPLAY);
        }
    }
    result
}

/// Generate the default `.gitignore` content for the config directory.
pub fn generate_gitignore() -> String {
    [
        "# lazycurl: auto-generated gitignore",
        "# Environment files may contain secrets",
        "environments/",
        "",
        "# History contains request metadata",
        "history.jsonl",
        "",
        "# Logs may contain request data",
        "logs/",
        "",
    ]
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_replaces_value() {
        assert_eq!(redact("my-secret-token"), "••••••");
    }

    #[test]
    fn test_redact_empty_string() {
        assert_eq!(redact(""), "••••••");
    }

    #[test]
    fn test_redact_in_string_replaces_occurrences() {
        let input = "Authorization: Bearer my-secret-token";
        let result = redact_in_string(input, "my-secret-token");
        assert_eq!(result, "Authorization: Bearer ••••••");
    }

    #[test]
    fn test_redact_in_string_multiple_occurrences() {
        let input = "token=abc&verify=abc";
        let result = redact_in_string(input, "abc");
        assert_eq!(result, "token=••••••&verify=••••••");
    }

    #[test]
    fn test_redact_in_string_no_match() {
        let input = "no secrets here";
        let result = redact_in_string(input, "missing");
        assert_eq!(result, "no secrets here");
    }

    #[test]
    fn test_generate_gitignore() {
        let content = generate_gitignore();
        assert!(content.contains("environments/"));
        assert!(content.contains("history.jsonl"));
        assert!(content.contains("logs/"));
    }

    #[test]
    fn test_redact_secrets_in_url() {
        let url = "https://api.example.com/users?token=secret123";
        let secrets = vec!["secret123".to_string()];
        let result = redact_secrets(url, &secrets);
        assert_eq!(result, "https://api.example.com/users?token=••••••");
        assert!(!result.contains("secret123"));
    }

    #[test]
    fn test_redact_secrets_multiple() {
        let text = "user=admin pass=s3cret key=s3cret";
        let secrets = vec!["admin".to_string(), "s3cret".to_string()];
        let result = redact_secrets(text, &secrets);
        assert!(!result.contains("admin"));
        assert!(!result.contains("s3cret"));
    }
}
