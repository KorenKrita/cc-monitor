use std::collections::HashMap;
use std::sync::Mutex;

use crate::parser::ParsedRequest;

struct CodexSessionState {
    cwd: String,
    model: String,
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
                let entry = state.entry(file_id.to_string()).or_insert(CodexSessionState {
                    cwd: String::new(),
                    model: String::new(),
                });
                entry.cwd = cwd;
                None
            }
            "turn_context" => {
                if let Some(model) = value.get("model").and_then(|m| m.as_str()) {
                    let mut state = self.session_state.lock().ok()?;
                    let entry = state.entry(file_id.to_string()).or_insert(CodexSessionState {
                        cwd: String::new(),
                        model: String::new(),
                    });
                    entry.model = model.to_string();
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
                    project: session.cwd.clone(),
                    source: "codex".to_string(),
                })
            }
            _ => None,
        }
    }
}