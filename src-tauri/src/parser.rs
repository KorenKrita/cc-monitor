// src-tauri/src/parser.rs
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
}

pub struct SessionTracker {
    last_user_timestamps: Mutex<HashMap<String, String>>,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self { last_user_timestamps: Mutex::new(HashMap::new()) }
    }

    pub fn parse_line(&self, line: &str) -> Option<ParsedRequest> {
        let entry: JsonlEntry = serde_json::from_str(line).ok()?;
        let entry_type = entry.entry_type.as_deref()?;

        let session_id = entry.session_id.clone().unwrap_or_default();

        if entry_type == "user" {
            if let Some(ts) = &entry.timestamp {
                let mut map = self.last_user_timestamps.lock().ok()?;
                map.insert(session_id, ts.clone());
            }
            return None;
        }

        if entry_type != "assistant" {
            return None;
        }

        let message = entry.message?;
        let model = message.model?;
        let usage = message.usage?;
        let timestamp = entry.timestamp?;

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
            input_tokens: usage.input_tokens.unwrap_or(0),
            output_tokens: usage.output_tokens.unwrap_or(0),
            cache_creation_tokens: usage.cache_creation_input_tokens.unwrap_or(0),
            cache_read_tokens: usage.cache_read_input_tokens.unwrap_or(0),
            duration_ms,
        })
    }
}
