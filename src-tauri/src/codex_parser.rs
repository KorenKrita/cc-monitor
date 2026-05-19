use std::collections::HashMap;
use std::sync::Mutex;

use crate::parser::ParsedRequest;

#[derive(Default)]
struct CodexSessionState {
    cwd: String,
    model: String,
}

// Encode cwd path to match Claude's project format: /Users/foo/bar → -Users-foo-bar
fn encode_cwd_as_project(cwd: &str) -> String {
    cwd.replace('/', "-")
}

pub struct CodexSessionTracker {
    session_state: Mutex<HashMap<String, CodexSessionState>>,
}

impl CodexSessionTracker {
    pub fn new() -> Self {
        Self {
            session_state: Mutex::new(HashMap::new()),
        }
    }

    pub fn parse_line(&self, line: &str, file_id: &str) -> Option<ParsedRequest> {
        let value: serde_json::Value = serde_json::from_str(line).ok()?;
        let entry_type = value.get("type")?.as_str()?;

        match entry_type {
            "session_meta" => {
                let cwd = value.get("cwd")?.as_str()?.to_string();
                let mut state = self.session_state.lock().ok()?;
                state.entry(file_id.to_string()).or_default().cwd = cwd;
                if state.len() > 200 {
                    if let Some(oldest) = state.keys().next().cloned() {
                        state.remove(&oldest);
                    }
                }
                None
            }
            "turn_context" => {
                if let Some(model) = value.get("model").and_then(|m| m.as_str()) {
                    let mut state = self.session_state.lock().ok()?;
                    state.entry(file_id.to_string()).or_default().model = model.to_string();
                }
                None
            }
            "event_msg" => {
                let info = value.get("info")?;
                let last_usage = info.get("last_token_usage")?;

                let input_tokens = last_usage.get("input_tokens")?.as_i64().unwrap_or(0);
                let output_tokens = last_usage.get("output_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                let reasoning_tokens = last_usage.get("reasoning_output_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                let cached_input = last_usage.get("cached_input_tokens").and_then(|v| v.as_i64()).unwrap_or(0);

                let state = self.session_state.lock().ok()?;
                let session = state.get(file_id)?;

                if session.model.is_empty() {
                    return None;
                }

                let timestamp = chrono::Utc::now().to_rfc3339();

                Some(ParsedRequest {
                    timestamp,
                    model: session.model.clone(),
                    input_tokens,
                    output_tokens: output_tokens + reasoning_tokens,
                    cache_creation_tokens: 0,
                    cache_read_tokens: cached_input,
                    duration_ms: None,
                    project: encode_cwd_as_project(&session.cwd),
                    source: "codex".to_string(),
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_meta_stores_cwd() {
        let tracker = CodexSessionTracker::new();
        let line = r#"{"type":"session_meta","cwd":"/Users/test/project"}"#;
        let result = tracker.parse_line(line, "file1");
        assert!(result.is_none());
    }

    #[test]
    fn test_full_flow() {
        let tracker = CodexSessionTracker::new();

        tracker.parse_line(r#"{"type":"session_meta","cwd":"/Users/test/project"}"#, "f1");
        tracker.parse_line(r#"{"type":"turn_context","model":"o3-pro"}"#, "f1");

        let event = r#"{"type":"event_msg","info":{"last_token_usage":{"input_tokens":2000,"output_tokens":800,"reasoning_output_tokens":400,"cached_input_tokens":500}}}"#;
        let result = tracker.parse_line(event, "f1").unwrap();

        assert_eq!(result.model, "o3-pro");
        assert_eq!(result.input_tokens, 2000);
        assert_eq!(result.output_tokens, 1200);
        assert_eq!(result.cache_read_tokens, 500);
        assert_eq!(result.project, "-Users-test-project");
        assert_eq!(result.source, "codex");
    }

    #[test]
    fn test_no_model_returns_none() {
        let tracker = CodexSessionTracker::new();
        tracker.parse_line(r#"{"type":"session_meta","cwd":"/tmp"}"#, "f1");

        let event = r#"{"type":"event_msg","info":{"last_token_usage":{"input_tokens":100,"output_tokens":50}}}"#;
        assert!(tracker.parse_line(event, "f1").is_none());
    }

    #[test]
    fn test_multiple_sessions_isolated() {
        let tracker = CodexSessionTracker::new();

        tracker.parse_line(r#"{"type":"session_meta","cwd":"/project-a"}"#, "f1");
        tracker.parse_line(r#"{"type":"turn_context","model":"gpt-4o"}"#, "f1");

        tracker.parse_line(r#"{"type":"session_meta","cwd":"/project-b"}"#, "f2");
        tracker.parse_line(r#"{"type":"turn_context","model":"o3"}"#, "f2");

        let event = r#"{"type":"event_msg","info":{"last_token_usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let r1 = tracker.parse_line(event, "f1").unwrap();
        assert_eq!(r1.model, "gpt-4o");
        assert_eq!(r1.project, "-project-a");

        let r2 = tracker.parse_line(event, "f2").unwrap();
        assert_eq!(r2.model, "o3");
        assert_eq!(r2.project, "-project-b");
    }

    #[test]
    fn test_encode_cwd_as_project() {
        assert_eq!(encode_cwd_as_project("/Users/foo/bar"), "-Users-foo-bar");
        assert_eq!(encode_cwd_as_project("/tmp"), "-tmp");
        assert_eq!(encode_cwd_as_project(""), "");
    }
}