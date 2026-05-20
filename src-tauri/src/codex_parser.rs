use std::collections::HashMap;
use std::sync::Mutex;

use crate::parser::ParsedRequest;

#[derive(Default)]
struct CodexSessionState {
    cwd: String,
    model: String,
    last_input: i64,
    last_output: i64,
    turn_start: Option<String>,
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
        let payload = value.get("payload")?;

        match entry_type {
            "session_meta" => {
                let cwd = payload.get("cwd")?.as_str()?.to_string();
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
                if let Some(model) = payload.get("model").and_then(|m| m.as_str()) {
                    let mut state = self.session_state.lock().ok()?;
                    state.entry(file_id.to_string()).or_default().model = model.to_string();
                }
                None
            }
            "event_msg" => {
                let msg_type = payload.get("type")?.as_str()?;

                if msg_type == "user_message" {
                    if let Some(ts) = value.get("timestamp").and_then(|t| t.as_str()) {
                        let mut state = self.session_state.lock().ok()?;
                        if let Some(session) = state.get_mut(file_id) {
                            session.turn_start = Some(ts.to_string());
                        }
                    }
                    return None;
                }

                if msg_type != "token_count" {
                    return None;
                }

                let info = payload.get("info")?;
                let last_usage = info.get("last_token_usage")?;

                let input_tokens = last_usage.get("input_tokens")?.as_i64().unwrap_or(0);
                let output_tokens = last_usage.get("output_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                let reasoning_tokens = last_usage.get("reasoning_output_tokens").and_then(|v| v.as_i64()).unwrap_or(0);
                let cached_input = last_usage.get("cached_input_tokens").and_then(|v| v.as_i64()).unwrap_or(0);

                let mut state = self.session_state.lock().ok()?;
                let session = state.get_mut(file_id)?;

                if session.model.is_empty() {
                    return None;
                }

                // Skip duplicate token_count events
                if input_tokens == session.last_input && output_tokens == session.last_output {
                    return None;
                }
                session.last_input = input_tokens;
                session.last_output = output_tokens;

                let timestamp = value.get("timestamp")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let timestamp = if timestamp.is_empty() {
                    chrono::Utc::now().to_rfc3339()
                } else {
                    timestamp.clone()
                };

                let duration_ms = session.turn_start.as_ref().and_then(|start_ts| {
                    let start = chrono::DateTime::parse_from_rfc3339(start_ts).ok()?;
                    let end = chrono::DateTime::parse_from_rfc3339(&timestamp).ok()?;
                    let ms = end.signed_duration_since(start).num_milliseconds();
                    if ms > 0 { Some(ms) } else { None }
                });

                Some(ParsedRequest {
                    timestamp,
                    model: session.model.clone(),
                    input_tokens,
                    output_tokens: output_tokens + reasoning_tokens,
                    cache_creation_tokens: 0,
                    cache_read_tokens: cached_input,
                    duration_ms,
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
        let line = r#"{"type":"session_meta","payload":{"cwd":"/Users/test/project","id":"abc"}}"#;
        let result = tracker.parse_line(line, "file1");
        assert!(result.is_none());
    }

    #[test]
    fn test_full_flow() {
        let tracker = CodexSessionTracker::new();

        tracker.parse_line(r#"{"type":"session_meta","payload":{"cwd":"/Users/test/project","id":"abc"}}"#, "f1");
        tracker.parse_line(r#"{"type":"turn_context","payload":{"model":"o3-pro","turn_id":"t1","cwd":"/Users/test/project"}}"#, "f1");
        tracker.parse_line(r#"{"timestamp":"2026-05-20T03:25:34.000Z","type":"event_msg","payload":{"type":"user_message","message":"hello"}}"#, "f1");

        let event = r#"{"timestamp":"2026-05-20T03:25:43.671Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":2000,"output_tokens":800,"reasoning_output_tokens":400,"cached_input_tokens":500}}}}"#;
        let result = tracker.parse_line(event, "f1").unwrap();

        assert_eq!(result.model, "o3-pro");
        assert_eq!(result.input_tokens, 2000);
        assert_eq!(result.output_tokens, 1200);
        assert_eq!(result.cache_read_tokens, 500);
        assert_eq!(result.project, "-Users-test-project");
        assert_eq!(result.source, "codex");
        assert_eq!(result.timestamp, "2026-05-20T03:25:43.671Z");
        assert_eq!(result.duration_ms, Some(9671));
    }

    #[test]
    fn test_no_model_returns_none() {
        let tracker = CodexSessionTracker::new();
        tracker.parse_line(r#"{"type":"session_meta","payload":{"cwd":"/tmp","id":"x"}}"#, "f1");

        let event = r#"{"timestamp":"2026-05-20T03:25:43Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":50}}}}"#;
        assert!(tracker.parse_line(event, "f1").is_none());
    }

    #[test]
    fn test_non_token_event_ignored() {
        let tracker = CodexSessionTracker::new();
        tracker.parse_line(r#"{"type":"session_meta","payload":{"cwd":"/tmp","id":"x"}}"#, "f1");
        tracker.parse_line(r#"{"type":"turn_context","payload":{"model":"gpt-4o","turn_id":"t1","cwd":"/tmp"}}"#, "f1");

        let event = r#"{"timestamp":"2026-05-20T03:25:34Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#;
        assert!(tracker.parse_line(event, "f1").is_none());
    }

    #[test]
    fn test_multiple_sessions_isolated() {
        let tracker = CodexSessionTracker::new();

        tracker.parse_line(r#"{"type":"session_meta","payload":{"cwd":"/project-a","id":"a"}}"#, "f1");
        tracker.parse_line(r#"{"type":"turn_context","payload":{"model":"gpt-4o","turn_id":"t1","cwd":"/project-a"}}"#, "f1");

        tracker.parse_line(r#"{"type":"session_meta","payload":{"cwd":"/project-b","id":"b"}}"#, "f2");
        tracker.parse_line(r#"{"type":"turn_context","payload":{"model":"o3","turn_id":"t2","cwd":"/project-b"}}"#, "f2");

        let event = r#"{"timestamp":"2026-05-20T03:25:52Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":100,"output_tokens":50}}}}"#;

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