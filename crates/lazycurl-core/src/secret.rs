/// The redaction placeholder shown in place of secret values.
pub const REDACTED_DISPLAY: &str = "••••••";

/// The redaction placeholder used in persisted output (history, logs).
pub const REDACTED_LOG: &str = "[REDACTED]";

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

    #[test]
    fn test_redact_secrets_empty_secrets_list() {
        let result = redact_secrets("sensitive text", &[]);
        assert_eq!(result, "sensitive text");
    }

    #[test]
    fn test_redact_secrets_empty_secret_value() {
        let result = redact_secrets("text", &["".to_string()]);
        assert_eq!(result, "text");
    }

    #[test]
    fn test_redact_secrets_substring_match() {
        let result = redact_secrets("keyboard key", &["key".to_string()]);
        assert_eq!(result, "••••••board ••••••");
    }

    #[test]
    fn test_redact_secrets_overlapping() {
        let secrets = vec!["secret".to_string(), "secret123".to_string()];
        let result = redact_secrets("secret123", &secrets);
        // "secret" is replaced first, turning "secret123" into "••••••123",
        // then "secret123" no longer matches, so the result keeps "123".
        assert!(!result.contains("secret123"));
        assert_eq!(result, "••••••123");
    }

    #[test]
    fn test_redact_secrets_no_match() {
        let result = redact_secrets("nothing here", &["missing".to_string()]);
        assert_eq!(result, "nothing here");
    }
}
