use crate::secret;
use crate::types::HistoryEntry;
use std::path::Path;

/// Append a history entry to the JSONL file.
pub fn append_entry(path: &Path, entry: &HistoryEntry) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(entry)?;
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", line)?;
    Ok(())
}

/// Append a history entry with secret values redacted.
pub fn append_entry_redacted(
    path: &Path,
    entry: &HistoryEntry,
    secrets: &[String],
) -> Result<(), Box<dyn std::error::Error>> {
    let mut redacted = entry.clone();
    redacted.url = secret::redact_secrets(&redacted.url, secrets)
        .replace(secret::REDACTED_DISPLAY, secret::REDACTED_LOG);
    append_entry(path, &redacted)
}

/// Prune history file to keep only the most recent `max_entries` entries.
pub fn prune_history(path: &Path, max_entries: usize) -> Result<(), Box<dyn std::error::Error>> {
    if !path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() <= max_entries {
        return Ok(());
    }

    let keep = &lines[lines.len() - max_entries..];
    let pruned = keep.join("\n") + "\n";
    std::fs::write(path, pruned)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HistoryEntry, Method};

    fn make_entry(name: &str) -> HistoryEntry {
        HistoryEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            collection_id: None,
            request_name: name.to_string(),
            method: Method::Get,
            url: "https://api.example.com/users".to_string(),
            status_code: Some(200),
            duration_ms: Some(142),
            environment: Some("Dev".to_string()),
            project_id: None,
            project_name: None,
        }
    }

    #[test]
    fn test_append_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("history.jsonl");

        append_entry(&path, &make_entry("Request 1")).unwrap();
        append_entry(&path, &make_entry("Request 2")).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_entry_contains_no_secrets() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("history.jsonl");

        let mut entry = make_entry("Secret Request");
        entry.url = "https://api.example.com/users?token=secret123".to_string();

        let secrets = vec!["secret123".to_string()];
        append_entry_redacted(&path, &entry, &secrets).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains("secret123"));
        assert!(content.contains("[REDACTED]"));
    }

    #[test]
    fn test_prune_respects_cap() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("history.jsonl");

        for i in 0..15 {
            append_entry(&path, &make_entry(&format!("Request {}", i))).unwrap();
        }

        prune_history(&path, 10).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 10);
        // Should keep the most recent entries
        assert!(content.contains("Request 14"));
        assert!(!content.contains("Request 0"));
    }

    #[test]
    fn test_append_to_nonexistent_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("subdir").join("history.jsonl");

        append_entry(&path, &make_entry("First")).unwrap();
        assert!(path.exists());
    }
}
