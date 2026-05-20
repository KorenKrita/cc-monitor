use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Deserialize)]
struct JsonlEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    timestamp: Option<String>,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    message: Option<MessagePayload>,
}

#[derive(Debug, Deserialize)]
struct MessagePayload {
    model: Option<String>,
    usage: Option<UsagePayload>,
}

#[derive(Debug, Deserialize)]
struct UsagePayload {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ParsedRequest {
    pub timestamp: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
    pub duration_ms: Option<i64>,
    pub project: String,
    pub source: String,
}

#[derive(Debug, Clone, Default)]
struct SessionUsage {
    input_tokens: i64,
    output_tokens: i64,
    cache_creation_tokens: i64,
    cache_read_tokens: i64,
}

pub struct SessionTracker {
    last_user_timestamps: Mutex<HashMap<String, String>>,
    last_usage: Mutex<HashMap<String, SessionUsage>>,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self {
            last_user_timestamps: Mutex::new(HashMap::new()),
            last_usage: Mutex::new(HashMap::new()),
        }
    }

    pub fn parse_line(&self, line: &str) -> Option<ParsedRequest> {
        let entry: JsonlEntry = serde_json::from_str(line).ok()?;
        let entry_type = entry.entry_type.as_deref()?;

        let session_id = entry.session_id.clone().unwrap_or_default();

        if entry_type == "user" {
            if let Some(ts) = &entry.timestamp {
                let mut map = self.last_user_timestamps.lock().ok()?;
                map.insert(session_id, ts.clone());
                if map.len() > 100 {
                    if let Some(oldest) = map.keys().next().cloned() {
                        map.remove(&oldest);
                    }
                }
            }
            return None;
        }

        if entry_type != "assistant" {
            return None;
        }

        let message = entry.message?;
        let model = message.model?;
        if model.starts_with('<') || model.is_empty() {
            return None;
        }
        let usage = message.usage?;
        let timestamp = entry.timestamp?;

        let cum_input = usage.input_tokens.unwrap_or(0);
        let cum_output = usage.output_tokens.unwrap_or(0);
        let cum_cache_creation = usage.cache_creation_input_tokens.unwrap_or(0);
        let cum_cache_read = usage.cache_read_input_tokens.unwrap_or(0);

        // Compute per-turn delta from cumulative session totals
        let (delta_input, delta_output, delta_cache_creation, delta_cache_read) = {
            let mut usage_map = self.last_usage.lock().ok()?;
            let prev = usage_map.entry(session_id.clone()).or_default();

            let di = cum_input - prev.input_tokens;
            let do_ = cum_output - prev.output_tokens;
            let dcc = cum_cache_creation - prev.cache_creation_tokens;
            let dcr = cum_cache_read - prev.cache_read_tokens;

            prev.input_tokens = cum_input;
            prev.output_tokens = cum_output;
            prev.cache_creation_tokens = cum_cache_creation;
            prev.cache_read_tokens = cum_cache_read;

            if usage_map.len() > 200 {
                if let Some(oldest) = usage_map.keys().next().cloned() {
                    usage_map.remove(&oldest);
                }
            }

            (di.max(0), do_.max(0), dcc.max(0), dcr.max(0))
        };

        // Skip duplicate entries (zero delta means same data repeated)
        if delta_input == 0 && delta_output == 0 && delta_cache_creation == 0 && delta_cache_read == 0 {
            return None;
        }

        let duration_ms = {
            let map = self.last_user_timestamps.lock().ok()?;
            map.get(&session_id).and_then(|user_ts| {
                let user_time = chrono::DateTime::parse_from_rfc3339(user_ts).ok()?;
                let assistant_time = chrono::DateTime::parse_from_rfc3339(&timestamp).ok()?;
                let duration = assistant_time.signed_duration_since(user_time);
                Some(duration.num_milliseconds())
            })
        };

        Some(ParsedRequest {
            timestamp,
            model,
            input_tokens: delta_input,
            output_tokens: delta_output,
            cache_creation_tokens: delta_cache_creation,
            cache_read_tokens: delta_cache_read,
            duration_ms,
            project: String::new(),
            source: String::new(),
        })
    }
}

impl From<crate::db::RequestRecord> for ParsedRequest {
    fn from(r: crate::db::RequestRecord) -> Self {
        Self {
            timestamp: r.timestamp,
            model: r.model,
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            cache_creation_tokens: r.cache_creation_tokens,
            cache_read_tokens: r.cache_read_tokens,
            duration_ms: r.duration_ms,
            project: r.project,
            source: r.source,
        }
    }
}

pub fn extract_project_from_claude_path(path: &std::path::Path) -> String {
    for ancestor in path.ancestors() {
        if let Some(parent) = ancestor.parent() {
            if parent.file_name().map_or(false, |n| n == "projects") {
                if let Some(gp) = parent.parent() {
                    if gp.file_name().map_or(false, |n| n == ".claude") {
                        return ancestor.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                    }
                }
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_project_from_claude_path() {
        let path = PathBuf::from("/Users/korenkrita/.claude/projects/-Users-korenkrita-Coding-cc-monitor/sessions/abc.jsonl");
        assert_eq!(extract_project_from_claude_path(&path), "-Users-korenkrita-Coding-cc-monitor");
    }

    #[test]
    fn test_extract_project_nested_session() {
        let path = PathBuf::from("/home/user/.claude/projects/my-project/deep/nested/file.jsonl");
        assert_eq!(extract_project_from_claude_path(&path), "my-project");
    }

    #[test]
    fn test_extract_project_non_claude_path() {
        let path = PathBuf::from("/tmp/random/file.jsonl");
        assert_eq!(extract_project_from_claude_path(&path), "");
    }

    #[test]
    fn test_parse_user_then_assistant_delta() {
        let tracker = SessionTracker::new();

        let user_line = r#"{"type":"user","timestamp":"2026-05-20T10:00:00Z","sessionId":"s1","message":{}}"#;
        assert!(tracker.parse_line(user_line).is_none());

        let assistant_line = r#"{"type":"assistant","timestamp":"2026-05-20T10:00:05Z","sessionId":"s1","message":{"model":"claude-opus-4-7","usage":{"input_tokens":1000,"output_tokens":500,"cache_creation_input_tokens":100,"cache_read_input_tokens":200}}}"#;
        let result = tracker.parse_line(assistant_line).unwrap();

        assert_eq!(result.model, "claude-opus-4-7");
        assert_eq!(result.input_tokens, 1000);
        assert_eq!(result.output_tokens, 500);
        assert_eq!(result.cache_creation_tokens, 100);
        assert_eq!(result.cache_read_tokens, 200);
        assert_eq!(result.duration_ms, Some(5000));
    }

    #[test]
    fn test_cumulative_to_delta() {
        let tracker = SessionTracker::new();

        // First assistant message: cumulative 1000 input, 500 output
        let line1 = r#"{"type":"assistant","timestamp":"2026-05-20T10:00:05Z","sessionId":"s1","message":{"model":"claude-opus-4-7","usage":{"input_tokens":1000,"output_tokens":500}}}"#;
        let r1 = tracker.parse_line(line1).unwrap();
        assert_eq!(r1.input_tokens, 1000);
        assert_eq!(r1.output_tokens, 500);

        // Second assistant message: cumulative 2500 input, 800 output → delta 1500, 300
        let line2 = r#"{"type":"assistant","timestamp":"2026-05-20T10:00:10Z","sessionId":"s1","message":{"model":"claude-opus-4-7","usage":{"input_tokens":2500,"output_tokens":800}}}"#;
        let r2 = tracker.parse_line(line2).unwrap();
        assert_eq!(r2.input_tokens, 1500);
        assert_eq!(r2.output_tokens, 300);
    }

    #[test]
    fn test_duplicate_skipped() {
        let tracker = SessionTracker::new();

        let line = r#"{"type":"assistant","timestamp":"2026-05-20T10:00:05Z","sessionId":"s1","message":{"model":"claude-opus-4-7","usage":{"input_tokens":1000,"output_tokens":500}}}"#;
        assert!(tracker.parse_line(line).is_some());
        // Same cumulative values = zero delta = skipped
        assert!(tracker.parse_line(line).is_none());
    }

    #[test]
    fn test_separate_sessions_independent() {
        let tracker = SessionTracker::new();

        let s1 = r#"{"type":"assistant","timestamp":"2026-05-20T10:00:05Z","sessionId":"s1","message":{"model":"claude-opus-4-7","usage":{"input_tokens":1000,"output_tokens":500}}}"#;
        let s2 = r#"{"type":"assistant","timestamp":"2026-05-20T10:00:05Z","sessionId":"s2","message":{"model":"claude-opus-4-7","usage":{"input_tokens":2000,"output_tokens":300}}}"#;

        let r1 = tracker.parse_line(s1).unwrap();
        let r2 = tracker.parse_line(s2).unwrap();
        assert_eq!(r1.input_tokens, 1000);
        assert_eq!(r2.input_tokens, 2000);
    }

    #[test]
    fn test_parse_assistant_without_user() {
        let tracker = SessionTracker::new();
        let line = r#"{"type":"assistant","timestamp":"2026-05-20T10:00:05Z","sessionId":"s1","message":{"model":"claude-opus-4-7","usage":{"input_tokens":1000,"output_tokens":500}}}"#;
        let result = tracker.parse_line(line).unwrap();
        assert_eq!(result.duration_ms, None);
    }

    #[test]
    fn test_parse_ignores_non_assistant() {
        let tracker = SessionTracker::new();
        let line = r#"{"type":"system","timestamp":"2026-05-20T10:00:00Z","sessionId":"s1","message":{}}"#;
        assert!(tracker.parse_line(line).is_none());
    }
}
