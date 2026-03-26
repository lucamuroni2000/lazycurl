use crate::config::config_dir;
use crate::secret;
use crate::types::{RequestLogEntry, ResponseLogData};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Error type for logging operations.
#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, LoggingError>;

/// Returns the directory where log files are stored.
pub fn logs_dir() -> PathBuf {
    config_dir().join("logs")
}

/// Redact secrets in a string for log output.
/// Replaces display redaction markers (••••••) with the log redaction marker ([REDACTED]).
fn redact_for_log(text: &str, secrets: &[String]) -> String {
    let redacted = secret::redact_secrets(text, secrets);
    redacted.replace(secret::REDACTED_DISPLAY, secret::REDACTED_LOG)
}

/// Write a request log entry to the daily log file.
///
/// - Redacts all secret values from the entry before writing.
/// - Truncates response body if it exceeds `max_body_size`.
/// - Appends a JSONL line to `<logs_path>/requests-YYYY-MM-DD.jsonl`.
pub fn log_request(
    logs_path: &Path,
    entry: &RequestLogEntry,
    secrets: &[String],
    max_body_size: usize,
) -> Result<()> {
    std::fs::create_dir_all(logs_path)?;

    let mut entry = entry.clone();

    // Redact request fields
    entry.request.url = redact_for_log(&entry.request.url, secrets);
    for header in &mut entry.request.headers {
        header.value = redact_for_log(&header.value, secrets);
    }
    if let Some(body) = &entry.request.body {
        entry.request.body = Some(redact_for_log(body, secrets));
    }

    // Redact and possibly truncate response fields
    if let Some(ref mut response) = entry.response {
        redact_response(response, secrets, max_body_size);
    }

    // Redact curl command
    entry.curl_command = redact_for_log(&entry.curl_command, secrets);

    // Determine filename from entry timestamp
    let date = entry.timestamp.format("%Y-%m-%d").to_string();
    let filename = format!("requests-{}.jsonl", date);
    let file_path = logs_path.join(filename);

    let json = serde_json::to_string(&entry)?;

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&file_path)?;
    writeln!(file, "{}", json)?;

    Ok(())
}

/// Redact secrets and truncate body in a response log entry.
fn redact_response(response: &mut ResponseLogData, secrets: &[String], max_body_size: usize) {
    for header in &mut response.headers {
        header.value = redact_for_log(&header.value, secrets);
    }
    if let Some(body) = &response.body {
        let redacted = redact_for_log(body, secrets);
        if redacted.len() > max_body_size {
            response.body = Some(redacted[..max_body_size].to_string());
            response.body_truncated = true;
        } else {
            response.body = Some(redacted);
        }
    }
}

/// Read request log entries from the logs directory.
///
/// If `date` is `Some("YYYY-MM-DD")`, reads only that date's file.
/// If `date` is `None`, reads all request log files (all dates).
pub fn read_request_logs(logs_path: &Path, date: Option<&str>) -> Result<Vec<RequestLogEntry>> {
    if !logs_path.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();

    let files: Vec<PathBuf> = if let Some(d) = date {
        let file = logs_path.join(format!("requests-{}.jsonl", d));
        if file.exists() {
            vec![file]
        } else {
            vec![]
        }
    } else {
        // Collect all request log files
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(logs_path)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("requests-") && name_str.ends_with(".jsonl") {
                paths.push(entry.path());
            }
        }
        paths.sort();
        paths
    };

    for file_path in files {
        let file = std::fs::File::open(&file_path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: RequestLogEntry = serde_json::from_str(&line)?;
            entries.push(entry);
        }
    }

    Ok(entries)
}

/// Delete log files older than `retention_days` days.
///
/// Parses dates from filenames of the form `requests-YYYY-MM-DD.jsonl`
/// and `debug-YYYY-MM-DD.log`. Files with dates strictly before the
/// cutoff are deleted.
pub fn cleanup_expired_logs(logs_path: &Path, retention_days: u32) -> Result<()> {
    if !logs_path.exists() {
        return Ok(());
    }

    let cutoff = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(retention_days as i64))
        .unwrap_or(chrono::Utc::now());
    let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

    for entry in std::fs::read_dir(logs_path)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        let date_str = if name_str.starts_with("requests-") && name_str.ends_with(".jsonl") {
            name_str
                .strip_prefix("requests-")
                .and_then(|s| s.strip_suffix(".jsonl"))
                .map(|s| s.to_string())
        } else if name_str.starts_with("debug-") && name_str.ends_with(".log") {
            name_str
                .strip_prefix("debug-")
                .and_then(|s| s.strip_suffix(".log"))
                .map(|s| s.to_string())
        } else {
            None
        };

        if let Some(date_str) = date_str {
            // Lexicographic comparison works for YYYY-MM-DD format
            if date_str < cutoff_str {
                std::fs::remove_file(entry.path())?;
            }
        }
    }

    Ok(())
}

/// Returns a sorted list of dates (most recent first) for which request log files exist.
pub fn available_log_dates(logs_path: &Path) -> Result<Vec<String>> {
    if !logs_path.exists() {
        return Ok(Vec::new());
    }

    let mut dates = Vec::new();

    for entry in std::fs::read_dir(logs_path)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with("requests-") && name_str.ends_with(".jsonl") {
            if let Some(date) = name_str
                .strip_prefix("requests-")
                .and_then(|s| s.strip_suffix(".jsonl"))
            {
                dates.push(date.to_string());
            }
        }
    }

    // Sort descending (most recent first)
    dates.sort_by(|a, b| b.cmp(a));

    Ok(dates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{LogHeader, Method, RequestLogData, RequestLogEntry, ResponseLogData};

    fn make_test_entry() -> RequestLogEntry {
        RequestLogEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            project: Some("test-project".to_string()),
            collection: Some("test-collection".to_string()),
            request: RequestLogData {
                method: Method::Post,
                url: "https://api.example.com/login".to_string(),
                url_template: Some("https://{{base_url}}/login".to_string()),
                headers: vec![
                    LogHeader {
                        name: "Content-Type".to_string(),
                        value: "application/json".to_string(),
                        value_template: None,
                    },
                    LogHeader {
                        name: "Authorization".to_string(),
                        value: "Bearer secret-token-123".to_string(),
                        value_template: Some("Bearer {{api_token}}".to_string()),
                    },
                ],
                body: Some(r#"{"password": "secret-token-123"}"#.to_string()),
                body_template: Some(r#"{"password": "{{api_token}}"}"#.to_string()),
                params: vec![],
            },
            response: Some(ResponseLogData {
                status_code: 200,
                status_text: "OK".to_string(),
                headers: vec![],
                body: Some(r#"{"token": "secret-token-123"}"#.to_string()),
                body_size_bytes: 30,
                body_truncated: false,
                body_type: "text".to_string(),
                time_ms: 142,
            }),
            curl_command: "curl -X POST https://api.example.com/login -H 'Authorization: Bearer secret-token-123'".to_string(),
            error: None,
        }
    }

    #[test]
    fn test_logs_dir_path() {
        let dir = logs_dir();
        let dir_str = dir.to_string_lossy();
        assert!(dir_str.contains("lazycurl"), "expected 'lazycurl' in path: {}", dir_str);
        assert!(dir.ends_with("logs"), "expected path to end with 'logs': {}", dir_str);
    }

    #[test]
    fn test_log_request_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let logs_path = tmp.path().join("logs");

        let entry = make_test_entry();
        let secrets = vec!["secret-token-123".to_string()];
        log_request(&logs_path, &entry, &secrets, 1024 * 1024).unwrap();

        let date = entry.timestamp.format("%Y-%m-%d").to_string();
        let expected_file = logs_path.join(format!("requests-{}.jsonl", date));
        assert!(expected_file.exists(), "expected log file to exist: {:?}", expected_file);
    }

    #[test]
    fn test_log_request_redacts_secrets() {
        let tmp = tempfile::tempdir().unwrap();
        let logs_path = tmp.path().join("logs");

        let entry = make_test_entry();
        let secrets = vec!["secret-token-123".to_string()];
        log_request(&logs_path, &entry, &secrets, 1024 * 1024).unwrap();

        let date = entry.timestamp.format("%Y-%m-%d").to_string();
        let file_path = logs_path.join(format!("requests-{}.jsonl", date));
        let content = std::fs::read_to_string(&file_path).unwrap();

        assert!(
            !content.contains("secret-token-123"),
            "log file should not contain the secret value"
        );
        assert!(
            content.contains("[REDACTED]"),
            "log file should contain [REDACTED] placeholder"
        );
        assert!(
            !content.contains("••••••"),
            "log file should not contain display redaction marker"
        );
    }

    #[test]
    fn test_log_request_appends_multiple() {
        let tmp = tempfile::tempdir().unwrap();
        let logs_path = tmp.path().join("logs");

        let entry = make_test_entry();
        let secrets = vec![];
        log_request(&logs_path, &entry, &secrets, 1024 * 1024).unwrap();
        log_request(&logs_path, &entry, &secrets, 1024 * 1024).unwrap();

        let date = entry.timestamp.format("%Y-%m-%d").to_string();
        let file_path = logs_path.join(format!("requests-{}.jsonl", date));
        let content = std::fs::read_to_string(&file_path).unwrap();

        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2, "expected 2 JSONL lines, got {}", lines.len());
    }

    #[test]
    fn test_log_request_truncates_large_body() {
        let tmp = tempfile::tempdir().unwrap();
        let logs_path = tmp.path().join("logs");

        let mut entry = make_test_entry();
        let large_body = "x".repeat(1000);
        if let Some(ref mut response) = entry.response {
            response.body = Some(large_body.clone());
            response.body_truncated = false;
        }

        let max_body_size = 100;
        let secrets = vec![];
        log_request(&logs_path, &entry, &secrets, max_body_size).unwrap();

        let date = entry.timestamp.format("%Y-%m-%d").to_string();
        let file_path = logs_path.join(format!("requests-{}.jsonl", date));
        let content = std::fs::read_to_string(&file_path).unwrap();

        let logged: RequestLogEntry = serde_json::from_str(content.trim()).unwrap();
        let response = logged.response.unwrap();
        assert!(
            response.body_truncated,
            "body_truncated should be true for large body"
        );
        assert_eq!(
            response.body.unwrap().len(),
            max_body_size,
            "body should be truncated to max_body_size"
        );
    }

    #[test]
    fn test_read_request_logs_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let logs_path = tmp.path().join("logs");

        let entries = read_request_logs(&logs_path, None).unwrap();
        assert!(entries.is_empty(), "expected empty vec for non-existent logs dir");
    }

    #[test]
    fn test_read_request_logs_returns_entries() {
        let tmp = tempfile::tempdir().unwrap();
        let logs_path = tmp.path().join("logs");

        let entry = make_test_entry();
        let secrets = vec![];
        log_request(&logs_path, &entry, &secrets, 1024 * 1024).unwrap();
        log_request(&logs_path, &entry, &secrets, 1024 * 1024).unwrap();

        let entries = read_request_logs(&logs_path, None).unwrap();
        assert_eq!(entries.len(), 2, "expected 2 log entries, got {}", entries.len());
    }

    #[test]
    fn test_read_request_logs_with_date_filter() {
        let tmp = tempfile::tempdir().unwrap();
        let logs_path = tmp.path().join("logs");

        let entry = make_test_entry();
        let secrets = vec![];
        log_request(&logs_path, &entry, &secrets, 1024 * 1024).unwrap();

        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let entries_today = read_request_logs(&logs_path, Some(&today)).unwrap();
        assert_eq!(entries_today.len(), 1, "expected 1 entry for today");

        let entries_wrong = read_request_logs(&logs_path, Some("1900-01-01")).unwrap();
        assert!(entries_wrong.is_empty(), "expected empty vec for wrong date");
    }

    #[test]
    fn test_cleanup_expired_logs() {
        let tmp = tempfile::tempdir().unwrap();
        let logs_path = tmp.path().join("logs");
        std::fs::create_dir_all(&logs_path).unwrap();

        // Create an old log file (30 days ago)
        let old_date = (chrono::Utc::now() - chrono::Duration::days(30))
            .format("%Y-%m-%d")
            .to_string();
        let old_file = logs_path.join(format!("requests-{}.jsonl", old_date));
        std::fs::write(&old_file, "old entry\n").unwrap();

        // Create today's log file
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let today_file = logs_path.join(format!("requests-{}.jsonl", today));
        std::fs::write(&today_file, "today entry\n").unwrap();

        // Cleanup with 7-day retention — old file should be deleted, today's kept
        cleanup_expired_logs(&logs_path, 7).unwrap();

        assert!(!old_file.exists(), "old log file should have been deleted");
        assert!(today_file.exists(), "today's log file should still exist");
    }

    #[test]
    fn test_available_log_dates() {
        let tmp = tempfile::tempdir().unwrap();
        let logs_path = tmp.path().join("logs");
        std::fs::create_dir_all(&logs_path).unwrap();

        // Create request log files
        std::fs::write(logs_path.join("requests-2026-03-24.jsonl"), "").unwrap();
        std::fs::write(logs_path.join("requests-2026-03-25.jsonl"), "").unwrap();
        std::fs::write(logs_path.join("requests-2026-03-26.jsonl"), "").unwrap();

        // Create a debug log file — should NOT appear in results
        std::fs::write(logs_path.join("debug-2026-03-26.log"), "").unwrap();

        let dates = available_log_dates(&logs_path).unwrap();

        assert_eq!(dates.len(), 3, "expected 3 dates, got {:?}", dates);
        assert_eq!(dates[0], "2026-03-26", "most recent date should be first");
        assert_eq!(dates[1], "2026-03-25");
        assert_eq!(dates[2], "2026-03-24");
        assert!(
            !dates.contains(&"debug-2026-03-26".to_string()),
            "debug files should not appear in dates"
        );
    }
}
