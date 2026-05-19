# Cost Tracking + Codex Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add real-time cost tracking to the status bar with configurable price sources, and extend the watcher to also monitor Codex CLI sessions.

**Architecture:** Extend the existing polling watcher to also scan `~/.codex/sessions/`. Add a `project` and `source` column to the DB. Cost is calculated in real-time from stored token counts × current price table (stored in settings.json). A new `cost` tray item type displays cumulative spend for a configurable time window.

**Tech Stack:** Rust (Tauri 2, rusqlite, reqwest for HTTP), React 19 + TypeScript, existing ECharts frontend.

---

### Task 1: Database Schema Migration — Add `project` and `source` columns

**Files:**
- Modify: `src-tauri/src/db.rs:24-44` (schema init)
- Modify: `src-tauri/src/db.rs:49-66` (insert_request)
- Modify: `src-tauri/src/db.rs:68-116` (query_requests)
- Modify: `src-tauri/src/db.rs:1-17` (RequestRecord struct)

- [ ] **Step 1: Add migration SQL to Database::new()**

In `db.rs`, after the existing `CREATE TABLE` and `CREATE INDEX` statements, add ALTER TABLE migrations wrapped in try (SQLite ignores duplicate column adds if we catch the error):

```rust
// After the execute_batch for CREATE TABLE, add:
let _ = conn.execute("ALTER TABLE requests ADD COLUMN project TEXT DEFAULT ''", []);
let _ = conn.execute("ALTER TABLE requests ADD COLUMN source TEXT DEFAULT 'claude'", []);
let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_project ON requests(project)", []);
let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_source ON requests(source)", []);
```

- [ ] **Step 2: Update RequestRecord struct**

```rust
#[derive(Debug, Clone, Serialize)]
pub struct RequestRecord {
    pub id: i64,
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
```

- [ ] **Step 3: Update insert_request signature and SQL**

```rust
pub fn insert_request(
    &self,
    timestamp: &str,
    model: &str,
    input_tokens: i64,
    output_tokens: i64,
    cache_creation_tokens: i64,
    cache_read_tokens: i64,
    duration_ms: Option<i64>,
    project: &str,
    source: &str,
) -> Result<i64, String> {
    let conn = self.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO requests (timestamp, model, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, duration_ms, project, source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![timestamp, model, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, duration_ms, project, source],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}
```

- [ ] **Step 4: Update query_requests row mapping**

Update the row mapping in `query_requests` and `get_latest` to include the new columns:

```rust
Ok(RequestRecord {
    id: row.get(0)?,
    timestamp: row.get(1)?,
    model: row.get(2)?,
    input_tokens: row.get(3)?,
    output_tokens: row.get(4)?,
    cache_creation_tokens: row.get(5)?,
    cache_read_tokens: row.get(6)?,
    duration_ms: row.get(7)?,
    project: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
    source: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
})
```

Update the SELECT statements to include `project, source`:
```sql
SELECT id, timestamp, model, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, duration_ms, project, source FROM requests ...
```

- [ ] **Step 5: Add delete_by_model and delete_all methods**

```rust
pub fn delete_by_model(&self, model: &str) -> Result<u64, String> {
    let conn = self.conn.lock().map_err(|e| e.to_string())?;
    let count = conn.execute("DELETE FROM requests WHERE model = ?1", params![model])
        .map_err(|e| e.to_string())?;
    Ok(count as u64)
}

pub fn delete_all(&self) -> Result<u64, String> {
    let conn = self.conn.lock().map_err(|e| e.to_string())?;
    let count = conn.execute("DELETE FROM requests", [])
        .map_err(|e| e.to_string())?;
    Ok(count as u64)
}
```

- [ ] **Step 6: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: Compilation errors in main.rs and parser.rs (insert_request call sites need updating) — that's expected, we fix those in later tasks.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/db.rs
git commit -m "feat: add project/source columns and delete methods to DB"
```

---

### Task 2: Update ParsedRequest and Claude Parser for Project Path

**Files:**
- Modify: `src-tauri/src/parser.rs:29-37` (ParsedRequest struct)
- Modify: `src-tauri/src/parser.rs:102-114` (From impl)
- Modify: `src-tauri/src/watcher.rs:63-85` (pass project to ParsedRequest)

- [ ] **Step 1: Add project and source fields to ParsedRequest**

```rust
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
```

- [ ] **Step 2: Add project path extraction helper**

Add to `parser.rs`:

```rust
pub fn extract_project_from_claude_path(path: &std::path::Path) -> String {
    for ancestor in path.ancestors() {
        if let Some(parent) = ancestor.parent() {
            if parent.ends_with("projects") && parent.parent().map_or(false, |p| p.ends_with(".claude")) {
                let dir_name = ancestor.file_name().unwrap_or_default().to_string_lossy();
                return dir_name.replace('-', "/").replacen("/", "", 0)
                    .trim_start_matches('/')
                    .to_string();
            }
        }
    }
    String::new()
}
```

Wait — the encoding is dashes replacing path separators. E.g. `-Users-korenkrita-Coding-cc-monitor` → `/Users/korenkrita/Coding/cc-monitor`. The first char is always `-` representing the leading `/`. So:

```rust
pub fn extract_project_from_claude_path(path: &std::path::Path) -> String {
    for ancestor in path.ancestors() {
        if let Some(parent) = ancestor.parent() {
            if parent.file_name().map_or(false, |n| n == "projects") {
                if let Some(gp) = parent.parent() {
                    if gp.file_name().map_or(false, |n| n == ".claude") {
                        let dir_name = ancestor.file_name().unwrap_or_default().to_string_lossy();
                        return dir_name.replacen('-', "/", dir_name.len());
                    }
                }
            }
        }
    }
    String::new()
}
```

Hmm, that's not right either — we can't just replace all dashes because project names might contain dashes. Looking at the actual encoding: the path `/Users/korenkrita/Coding/cc-monitor` becomes `-Users-korenkrita-Coding-cc-monitor`. The separator is `-` but it's ambiguous with hyphens in names.

Actually looking at Claude Code's source, the encoding replaces `/` with `-`. So `/Users/korenkrita/Coding/cc-monitor` → `-Users-korenkrita-Coding-cc-monitor`. To decode, we need to know the actual filesystem paths. The simplest reliable approach: just store the encoded directory name as-is and let the user configure their whitelist using the same encoded format. Or better: check which decoded path actually exists on disk.

Simpler approach — store the raw directory name (e.g. `-Users-korenkrita-Coding-cc-monitor`) and display/filter using that. The user sees these in the UI and can whitelist them directly.

```rust
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
```

- [ ] **Step 3: Update From<RequestRecord> impl**

```rust
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
```

- [ ] **Step 4: Update watcher to pass file path for project extraction**

In `watcher.rs`, change `parse_line` call to also pass the file path. But since `parse_line` is on `SessionTracker`, we need a new method or pass project separately. Simplest: extract project in the watcher and attach it after parsing.

Change the watcher loop (lines 63-85) to:

```rust
if let Some(mut request) = tracker.parse_line(trimmed) {
    request.project = crate::parser::extract_project_from_claude_path(path);
    request.source = "claude".to_string();
    let _ = tx.send(request);
}
```

And update `parse_line` to set default empty values:

In `parse_line`, change the `Some(ParsedRequest { ... })` return to include:
```rust
project: String::new(),
source: String::new(),
```

- [ ] **Step 5: Update main.rs insert_request call**

In `main.rs` line 97-105, update the insert call:

```rust
let _ = db_clone.insert_request(
    &request.timestamp,
    &request.model,
    request.input_tokens,
    request.output_tokens,
    request.cache_creation_tokens,
    request.cache_read_tokens,
    request.duration_ms,
    &request.project,
    &request.source,
);
```

- [ ] **Step 6: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: PASS (all call sites updated)

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/parser.rs src-tauri/src/watcher.rs src-tauri/src/main.rs
git commit -m "feat: track project path and source in parsed requests"
```

---

### Task 3: Codex CLI Parser

**Files:**
- Create: `src-tauri/src/codex_parser.rs`
- Modify: `src-tauri/src/lib.rs` (add module declaration)

- [ ] **Step 1: Create codex_parser.rs with JSONL structs**

```rust
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Deserialize)]
struct CodexEntry {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    #[serde(flatten)]
    data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct SessionMeta {
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TurnContext {
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenCountInfo {
    last_token_usage: Option<TokenUsage>,
}

#[derive(Debug, Deserialize)]
struct TokenUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cached_input_tokens: Option<i64>,
    reasoning_output_tokens: Option<i64>,
}

use crate::parser::ParsedRequest;

pub struct CodexSessionTracker {
    session_state: Mutex<HashMap<String, CodexSessionState>>,
}

struct CodexSessionState {
    cwd: String,
    model: String,
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
                state.entry(file_id.to_string()).or_insert(CodexSessionState {
                    cwd,
                    model: String::new(),
                }).cwd = value.get("cwd")?.as_str()?.to_string();
                None
            }
            "turn_context" => {
                if let Some(model) = value.get("model").and_then(|m| m.as_str()) {
                    let mut state = self.session_state.lock().ok()?;
                    state.entry(file_id.to_string()).or_insert(CodexSessionState {
                        cwd: String::new(),
                        model: String::new(),
                    }).model = model.to_string();
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
```

- [ ] **Step 2: Add module declaration in lib.rs**

Add to `src-tauri/src/lib.rs`:
```rust
pub mod codex_parser;
```

(Or if lib.rs just re-exports main.rs modules, add `mod codex_parser;` to `main.rs` instead.)

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/codex_parser.rs src-tauri/src/lib.rs
git commit -m "feat: add Codex CLI JSONL parser"
```

---

### Task 4: Extend Watcher to Monitor Codex Sessions

**Files:**
- Modify: `src-tauri/src/watcher.rs`

- [ ] **Step 1: Add Codex directory scanning to the polling loop**

Refactor `start_polling` to watch both directories:

```rust
use crate::codex_parser::CodexSessionTracker;

pub fn start_polling(tx: mpsc::UnboundedSender<ParsedRequest>) {
    std::thread::spawn(move || {
        let claude_dir = dirs::home_dir().map(|h| h.join(".claude").join("projects"));
        let codex_dir = dirs::home_dir().map(|h| h.join(".codex").join("sessions"));

        let tracker = Arc::new(SessionTracker::new());
        let codex_tracker = Arc::new(CodexSessionTracker::new());
        let mut file_positions: HashMap<PathBuf, u64> = HashMap::new();

        // Seed Claude sessions (existing logic)
        if let Some(ref dir) = claude_dir {
            if dir.exists() {
                if let Ok(files) = glob_jsonl_files(dir) {
                    for path in &files {
                        if let Ok(meta) = std::fs::metadata(path) {
                            file_positions.insert(path.clone(), meta.len());
                        }
                    }
                    let mut recent: Vec<_> = files.iter()
                        .filter_map(|p| std::fs::metadata(p).ok().map(|m| (p, m.modified().ok())))
                        .filter_map(|(p, t)| t.map(|t| (p, t)))
                        .collect();
                    recent.sort_by(|a, b| b.1.cmp(&a.1));
                    for (path, _) in recent.iter().take(5) {
                        seed_last_user_timestamp(path, &tracker);
                    }
                }
            }
        }

        // Seed Codex positions
        if let Some(ref dir) = codex_dir {
            if dir.exists() {
                if let Ok(files) = glob_jsonl_files(dir) {
                    for path in &files {
                        if let Ok(meta) = std::fs::metadata(path) {
                            file_positions.insert(path.clone(), meta.len());
                        }
                    }
                }
            }
        }

        loop {
            std::thread::sleep(Duration::from_millis(500));

            // Scan Claude files
            if let Some(ref dir) = claude_dir {
                if dir.exists() {
                    if let Ok(files) = glob_jsonl_files(dir) {
                        for path in &files {
                            process_file(path, &mut file_positions, |line| {
                                if let Some(mut req) = tracker.parse_line(line) {
                                    req.project = crate::parser::extract_project_from_claude_path(path);
                                    req.source = "claude".to_string();
                                    let _ = tx.send(req);
                                }
                            });
                        }
                    }
                }
            }

            // Scan Codex files
            if let Some(ref dir) = codex_dir {
                if dir.exists() {
                    if let Ok(files) = glob_jsonl_files(dir) {
                        for path in &files {
                            let file_id = path.to_string_lossy().to_string();
                            process_file(path, &mut file_positions, |line| {
                                if let Some(req) = codex_tracker.parse_line(line, &file_id) {
                                    let _ = tx.send(req);
                                }
                            });
                        }
                    }
                }
            }
        }
    });
}

fn process_file<F>(path: &PathBuf, positions: &mut HashMap<PathBuf, u64>, mut handler: F)
where
    F: FnMut(&str),
{
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return,
    };

    let current_len = meta.len();
    let last_pos = positions.get(path).copied().unwrap_or(0);

    if current_len < last_pos {
        positions.insert(path.clone(), 0);
        return;
    }
    if current_len == last_pos {
        return;
    }

    if let Ok(file) = File::open(path) {
        let mut reader = BufReader::new(file);
        if reader.seek(SeekFrom::Start(last_pos)).is_ok() {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            handler(trimmed);
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }

    positions.insert(path.clone(), current_len);
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/watcher.rs
git commit -m "feat: extend watcher to monitor Codex CLI sessions"
```

---

### Task 5: Config Extension — CostConfig and Price Model

**Files:**
- Modify: `src-tauri/src/config.rs`
- Modify: `src/types.ts`

- [ ] **Step 1: Add CostConfig and ModelPrice structs to config.rs**

```rust
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPrice {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
    #[serde(default)]
    pub cache: f64,
    #[serde(default = "default_source")]
    pub source: String,
}

fn default_source() -> String { "manual".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostConfig {
    #[serde(default = "default_time_window")]
    pub time_window: String,
    #[serde(default)]
    pub project_whitelist: Vec<String>,
    #[serde(default)]
    pub model_whitelist: Vec<String>,
    #[serde(default)]
    pub model_prices: HashMap<String, ModelPrice>,
    #[serde(default)]
    pub last_sync_time: Option<String>,
}

fn default_time_window() -> String { "day".into() }

impl Default for CostConfig {
    fn default() -> Self {
        Self {
            time_window: default_time_window(),
            project_whitelist: vec![],
            model_whitelist: vec![],
            model_prices: HashMap::new(),
            last_sync_time: None,
        }
    }
}
```

- [ ] **Step 2: Add cost field to Config struct**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub tray: TrayConfig,
    #[serde(default)]
    pub model_aliases: HashMap<String, String>,
    #[serde(default)]
    pub cost: CostConfig,
}
```

Update `Default for Config`:
```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            tray: TrayConfig::default(),
            model_aliases: HashMap::new(),
            cost: CostConfig::default(),
        }
    }
}
```

- [ ] **Step 3: Update TypeScript types**

In `src/types.ts`:

```typescript
export type Metric = "out_rate" | "in_rate" | "ttft" | "cost";
export type CostTimeWindow = "day" | "month" | "year" | "all";

export interface ModelPrice {
  input: number;
  output: number;
  cache: number;
  source: string;
}

export interface CostConfig {
  time_window: CostTimeWindow;
  project_whitelist: string[];
  model_whitelist: string[];
  model_prices: Record<string, ModelPrice>;
  last_sync_time: string | null;
}

export interface Config {
  theme: Theme;
  tray: {
    items: Metric[];
    model_filter: "last" | "whitelist";
    model_whitelist: string[];
    display_mode: "last" | "average";
    average_minutes: number;
  };
  model_aliases: Record<string, string>;
  cost: CostConfig;
}

export interface RequestRecord {
  id: number;
  timestamp: string;
  model: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  duration_ms: number | null;
  project: string;
  source: string;
}
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/config.rs src/types.ts
git commit -m "feat: add CostConfig and ModelPrice to config schema"
```

---

### Task 6: Cost Calculation Logic + Tray Display

**Files:**
- Modify: `src-tauri/src/tray.rs` (add cost formatting)
- Modify: `src-tauri/src/db.rs` (add cost query method)
- Modify: `src-tauri/src/main.rs` (integrate cost into tray update loop)

- [ ] **Step 1: Add cost query method to Database**

In `db.rs`, add a method that calculates total cost for a time window:

```rust
pub fn calculate_cost(
    &self,
    since: &str,
    project_whitelist: &[String],
    model_whitelist: &[String],
    model_prices: &std::collections::HashMap<String, crate::config::ModelPrice>,
) -> Result<f64, String> {
    let conn = self.conn.lock().map_err(|e| e.to_string())?;
    let mut sql = "SELECT model, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, project FROM requests WHERE timestamp >= ?1".to_string();
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(since.to_string())];
    let mut param_idx = 2;

    if !project_whitelist.is_empty() {
        let placeholders: Vec<String> = project_whitelist.iter().enumerate()
            .map(|(i, _)| format!("?{}", i + param_idx)).collect();
        sql.push_str(&format!(" AND project IN ({})", placeholders.join(",")));
        param_idx += project_whitelist.len();
        for p in project_whitelist {
            param_values.push(Box::new(p.clone()));
        }
    }

    if !model_whitelist.is_empty() {
        let placeholders: Vec<String> = model_whitelist.iter().enumerate()
            .map(|(i, _)| format!("?{}", i + param_idx)).collect();
        sql.push_str(&format!(" AND model IN ({})", placeholders.join(",")));
        for m in model_whitelist {
            param_values.push(Box::new(m.clone()));
        }
    }

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

    let mut total_cost: f64 = 0.0;
    let mut rows = stmt.query(params_ref.as_slice()).map_err(|e| e.to_string())?;
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let model: String = row.get(0).map_err(|e| e.to_string())?;
        let input: i64 = row.get(1).map_err(|e| e.to_string())?;
        let output: i64 = row.get(2).map_err(|e| e.to_string())?;
        let cache_creation: i64 = row.get(3).map_err(|e| e.to_string())?;
        let cache_read: i64 = row.get(4).map_err(|e| e.to_string())?;

        if let Some(price) = model_prices.get(&model) {
            let cost = (input as f64 * price.input
                + output as f64 * price.output
                + (cache_creation + cache_read) as f64 * price.cache)
                / 1_000_000.0;
            total_cost += cost;
        }
    }

    Ok(total_cost)
}
```

- [ ] **Step 2: Add cost formatting to tray.rs**

```rust
pub fn format_cost(amount: f64) -> String {
    if amount >= 1000.0 {
        format!("${:.1}k", amount / 1000.0)
    } else {
        format!("${:.2}", amount)
    }
}

pub fn calculate_cost_since(time_window: &str) -> String {
    let now = chrono::Utc::now();
    match time_window {
        "day" => (now - chrono::Duration::days(1)).to_rfc3339(),
        "month" => (now - chrono::Duration::days(30)).to_rfc3339(),
        "year" => (now - chrono::Duration::days(365)).to_rfc3339(),
        "all" => "1970-01-01T00:00:00Z".to_string(),
        _ => (now - chrono::Duration::days(1)).to_rfc3339(),
    }
}
```

- [ ] **Step 3: Integrate cost into tray text formatting**

Update `format_tray_text` and `format_idle_tray_text` in `tray.rs` to handle the `"cost"` item:

In `format_tray_text`, add to the match:
```rust
"cost" => {} // cost is handled separately in the main loop since it needs DB access
```

In `format_idle_tray_text`, add:
```rust
"cost" => parts.push("$—".to_string()),
```

- [ ] **Step 4: Update main.rs tray update loop to include cost**

In the async block in `main.rs`, after computing `tray_text`, add cost calculation:

```rust
let tray_text = match current_config.tray.display_mode.as_str() {
    // ... existing logic ...
};

// Append cost if enabled
let tray_text = if current_config.tray.items.contains(&"cost".to_string()) {
    let since = tray::calculate_cost_since(&current_config.cost.time_window);
    let cost = db_clone.calculate_cost(
        &since,
        &current_config.cost.project_whitelist,
        &current_config.cost.model_whitelist,
        &current_config.cost.model_prices,
    ).unwrap_or(0.0);
    let cost_str = tray::format_cost(cost);

    // Insert cost at the correct position based on items order
    let mut parts: Vec<String> = Vec::new();
    for item in &current_config.tray.items {
        match item.as_str() {
            "cost" => parts.push(cost_str.clone()),
            _ => {} // other items already in tray_text
        }
    }
    // Rebuild: we need to restructure to build all parts together
    // Actually, let's refactor to build all parts in one pass
    tray_text // placeholder — see Step 5 for full refactor
} else {
    tray_text
};
```

Actually, the cleaner approach is to refactor `format_tray_text` to accept an optional cost value:

- [ ] **Step 5: Refactor format_tray_text to accept cost**

Update signature:
```rust
pub fn format_tray_text(request: &ParsedRequest, config: &TrayConfig, cost: Option<f64>) -> String {
    let mut parts: Vec<String> = Vec::new();
    let duration_s = request.duration_ms.filter(|&ms| ms > 0).map(|ms| ms as f64 / 1000.0);

    for item in &config.items {
        match item.as_str() {
            "out_rate" => {
                if let Some(ds) = duration_s {
                    let rate = (request.output_tokens as f64 / ds).round() as i64;
                    parts.push(format!("↓{}", format_rate(rate)));
                }
            }
            "in_rate" => {
                if let Some(ds) = duration_s {
                    let rate = (request.input_tokens as f64 / ds).round() as i64;
                    parts.push(format!("↑{}", format_rate(rate)));
                }
            }
            "ttft" => {
                if let Some(ms) = request.duration_ms.filter(|&ms| ms > 0) {
                    parts.push(format!("⏱{}", format_duration(ms)));
                } else {
                    parts.push("⏱—".to_string());
                }
            }
            "cost" => {
                if let Some(c) = cost {
                    parts.push(format_cost(c));
                } else {
                    parts.push("$—".to_string());
                }
            }
            _ => {}
        }
    }

    if parts.is_empty() {
        "✧".to_string()
    } else {
        format!("✧ {}", parts.join(" "))
    }
}
```

Do the same for `format_idle_tray_text` and `format_average_tray_text` — add `cost: Option<f64>` parameter and handle `"cost"` in the match.

- [ ] **Step 6: Update all call sites of format_tray_text**

In `main.rs` and `commands.rs`, pass the cost value:

```rust
// In main.rs tray update:
let cost = if current_config.tray.items.contains(&"cost".to_string()) {
    let since = tray::calculate_cost_since(&current_config.cost.time_window);
    db_clone.calculate_cost(
        &since,
        &current_config.cost.project_whitelist,
        &current_config.cost.model_whitelist,
        &current_config.cost.model_prices,
    ).ok()
} else {
    None
};

let tray_text = match current_config.tray.display_mode.as_str() {
    "average" => {
        let mins = current_config.tray.average_minutes.max(1);
        let since = chrono::Utc::now() - chrono::Duration::minutes(mins as i64);
        tray::format_average_tray_text(&db_clone, &since.to_rfc3339(), &current_config.tray, cost)
    }
    _ => {
        if request.duration_ms.filter(|&ms| ms > 0).is_some() {
            tray::format_tray_text(&request, &current_config.tray, cost)
        } else {
            continue;
        }
    }
};
```

- [ ] **Step 7: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/tray.rs src-tauri/src/db.rs src-tauri/src/main.rs src-tauri/src/commands.rs
git commit -m "feat: add cost calculation and tray display"
```

---

### Task 7: Price Sync — HTTP Fetching from Remote Sources

**Files:**
- Create: `src-tauri/src/price_sync.rs`
- Modify: `src-tauri/Cargo.toml` (add reqwest dependency)
- Modify: `src-tauri/src/main.rs` (add module)

- [ ] **Step 1: Add reqwest to Cargo.toml**

```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
```

- [ ] **Step 2: Create price_sync.rs**

```rust
use std::collections::HashMap;
use crate::config::ModelPrice;

const LITELLM_URL: &str = "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";

pub async fn fetch_litellm_prices() -> Result<HashMap<String, ModelPrice>, String> {
    let resp = reqwest::get(LITELLM_URL).await.map_err(|e| e.to_string())?;
    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let obj = data.as_object().ok_or("Invalid litellm format")?;
    let mut prices = HashMap::new();

    for (model_name, info) in obj {
        if model_name.starts_with("sample_spec") {
            continue;
        }
        let input = info.get("input_cost_per_token")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) * 1_000_000.0;
        let output = info.get("output_cost_per_token")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) * 1_000_000.0;
        let cache = info.get("cache_read_input_token_cost")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) * 1_000_000.0;

        if input > 0.0 || output > 0.0 {
            prices.insert(model_name.clone(), ModelPrice {
                input,
                output,
                cache,
                source: "litellm".to_string(),
            });
        }
    }

    Ok(prices)
}

pub async fn sync_prices(current_prices: &HashMap<String, ModelPrice>) -> Result<HashMap<String, ModelPrice>, String> {
    let mut result = current_prices.clone();

    let fetched = fetch_litellm_prices().await?;

    for (model, price) in fetched {
        match result.get(&model) {
            Some(existing) if existing.source == "manual" => {
                // Don't overwrite manual prices
            }
            _ => {
                result.insert(model, price);
            }
        }
    }

    Ok(result)
}
```

- [ ] **Step 3: Add module declaration**

In `main.rs`:
```rust
mod price_sync;
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/price_sync.rs src-tauri/src/main.rs
git commit -m "feat: add price sync from litellm"
```

---

### Task 8: New Tauri Commands — Cost, Sync, Delete

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs` (register commands)

- [ ] **Step 1: Add get_cost command**

```rust
#[tauri::command]
pub fn get_cost(state: State<AppState>) -> Result<f64, String> {
    let config = load_config();
    let since = crate::tray::calculate_cost_since(&config.cost.time_window);
    state.db.calculate_cost(
        &since,
        &config.cost.project_whitelist,
        &config.cost.model_whitelist,
        &config.cost.model_prices,
    )
}
```

- [ ] **Step 2: Add sync_prices command**

```rust
#[tauri::command]
pub async fn sync_prices() -> Result<Config, String> {
    let mut config = load_config();
    let synced = crate::price_sync::sync_prices(&config.cost.model_prices).await?;
    config.cost.model_prices = synced;
    config.cost.last_sync_time = Some(chrono::Utc::now().to_rfc3339());
    save_config(&config)?;
    Ok(config)
}
```

- [ ] **Step 3: Add delete commands**

```rust
#[tauri::command]
pub fn delete_model_data(state: State<AppState>, model: String) -> Result<u64, String> {
    state.db.delete_by_model(&model)
}

#[tauri::command]
pub fn delete_all_data(state: State<AppState>) -> Result<u64, String> {
    state.db.delete_all()
}
```

- [ ] **Step 4: Add get_projects command**

```rust
#[tauri::command]
pub fn get_projects(state: State<AppState>) -> Result<Vec<String>, String> {
    state.db.get_projects()
}
```

And add to `db.rs`:
```rust
pub fn get_projects(&self) -> Result<Vec<String>, String> {
    let conn = self.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT DISTINCT project FROM requests WHERE project != '' ORDER BY project")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}
```

- [ ] **Step 5: Register new commands in main.rs**

```rust
.invoke_handler(tauri::generate_handler![
    commands::get_requests,
    commands::get_latest,
    commands::get_models,
    commands::get_config,
    commands::set_config,
    commands::hide_window,
    commands::quit_app,
    commands::get_cost,
    commands::sync_prices,
    commands::delete_model_data,
    commands::delete_all_data,
    commands::get_projects,
])
```

- [ ] **Step 6: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs src-tauri/src/db.rs
git commit -m "feat: add cost, sync, and delete Tauri commands"
```

---

### Task 9: Frontend — Settings UI for Cost Configuration

**Files:**
- Modify: `src/components/Settings.tsx`

- [ ] **Step 1: Add Cost section to Settings component**

After the "Model Aliases" section, add a new "Cost" section. Insert before the footer buttons `</div>`:

```tsx
{/* Cost Configuration */}
<div>
  <div style={labelStyle}>Cost — Time Window</div>
  <select
    value={draft.cost.time_window}
    onChange={(e) => setDraft({ ...draft, cost: { ...draft.cost, time_window: e.target.value as CostTimeWindow } })}
    style={selectStyle}
  >
    <option value="day">Day</option>
    <option value="month">Month</option>
    <option value="year">Year</option>
    <option value="all">All Time</option>
  </select>
</div>

{/* Cost Project Whitelist */}
<div>
  <div style={labelStyle}>Cost — Project Whitelist</div>
  <textarea
    value={draft.cost.project_whitelist.join("\n")}
    onChange={(e) => {
      const list = e.target.value.split("\n").map((s) => s.trim()).filter(Boolean);
      setDraft({ ...draft, cost: { ...draft.cost, project_whitelist: list } });
    }}
    placeholder="Leave empty for all projects"
    style={{ ...inputStyle, height: 50, resize: "vertical" as const, fontFamily: "'Fira Code', monospace" }}
  />
  <div style={{ fontSize: 9, color: theme.muted, marginTop: 3 }}>
    One project per line. Empty = all projects counted.
  </div>
</div>

{/* Cost Model Whitelist */}
<div>
  <div style={labelStyle}>Cost — Model Whitelist</div>
  <textarea
    value={draft.cost.model_whitelist.join(", ")}
    onChange={(e) => {
      const list = e.target.value.split(",").map((s) => s.trim()).filter(Boolean);
      setDraft({ ...draft, cost: { ...draft.cost, model_whitelist: list } });
    }}
    placeholder="Leave empty for all models"
    style={{ ...inputStyle, height: 40, resize: "vertical" as const, fontFamily: "'Fira Code', monospace" }}
  />
  <div style={{ fontSize: 9, color: theme.muted, marginTop: 3 }}>
    Comma-separated. Empty = all models counted.
  </div>
</div>
```

- [ ] **Step 2: Add "cost" to tray items list**

Update the unchecked items list to include "cost":

```tsx
{(["out_rate", "in_rate", "ttft", "cost"] as Metric[]).filter((m) => !draft.tray.items.includes(m)).map((item) => (
  <div key={item} style={{ display: "flex", alignItems: "center", gap: 6, opacity: 0.5 }}>
    <input
      type="checkbox"
      checked={false}
      onChange={() => toggleTrayItem(item)}
      style={{ accentColor: theme.accentGreen }}
    />
    <span style={{ fontSize: 11 }}>
      {item === "out_rate" ? "↓ Out Rate" : item === "in_rate" ? "↑ In Rate" : item === "ttft" ? "⏱ TTFT" : "$ Cost"}
    </span>
  </div>
))}
```

Also update the checked items label rendering:
```tsx
<span style={{ fontSize: 11, flex: 1 }}>
  {item === "out_rate" ? "↓ Out Rate" : item === "in_rate" ? "↑ In Rate" : item === "ttft" ? "⏱ TTFT" : "$ Cost"}
</span>
```

- [ ] **Step 3: Add CostTimeWindow import**

At the top of Settings.tsx:
```tsx
import { Config, Theme, Metric, CostTimeWindow } from "../types";
```

- [ ] **Step 4: Verify dev server renders correctly**

Run: `npm run dev` (or `pnpm dev`)
Expected: Settings page shows new Cost sections without errors.

- [ ] **Step 5: Commit**

```bash
git add src/components/Settings.tsx
git commit -m "feat: add cost configuration UI to settings"
```

---

### Task 10: Frontend — Model Price Table UI

**Files:**
- Create: `src/components/PriceTable.tsx`
- Modify: `src/components/Settings.tsx` (import and render PriceTable)

- [ ] **Step 1: Create PriceTable component**

```tsx
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Config, ModelPrice } from "../types";
import { ThemeTokens } from "../theme";

interface Props {
  config: Config;
  models: string[];
  onUpdate: (prices: Record<string, ModelPrice>) => void;
  onSyncComplete: (config: Config) => void;
  theme: ThemeTokens;
}

export function PriceTable({ config, models, onUpdate, onSyncComplete, theme }: Props) {
  const [syncing, setSyncing] = useState(false);

  const prices = config.cost.model_prices;

  const updatePrice = (model: string, field: keyof ModelPrice, value: number) => {
    const current = prices[model] || { input: 0, output: 0, cache: 0, source: "manual" };
    const updated = { ...prices, [model]: { ...current, [field]: value, source: "manual" } };
    onUpdate(updated);
  };

  const handleSync = async () => {
    setSyncing(true);
    try {
      const newConfig = await invoke<Config>("sync_prices");
      onSyncComplete(newConfig);
    } catch (e) {
      console.error("Sync failed:", e);
    } finally {
      setSyncing(false);
    }
  };

  const allModels = [...new Set([...models, ...Object.keys(prices)])].sort();

  const cellStyle = {
    background: theme.card,
    border: `1px solid ${theme.border}`,
    borderRadius: 4,
    padding: "3px 6px",
    fontSize: 10,
    color: theme.foreground,
    outline: "none",
    width: 65,
    textAlign: "right" as const,
  };

  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 6 }}>
        <span style={{ fontSize: 9, color: theme.muted, textTransform: "uppercase", letterSpacing: 0.5 }}>
          Model Prices ($/M tokens)
        </span>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {config.cost.last_sync_time && (
            <span style={{ fontSize: 9, color: theme.muted }}>
              Last sync: {new Date(config.cost.last_sync_time).toLocaleDateString()}
            </span>
          )}
          <button
            onClick={handleSync}
            disabled={syncing}
            style={{
              background: theme.accentGreen, border: "none", borderRadius: 4,
              color: "#fff", fontSize: 10, padding: "3px 8px", cursor: syncing ? "wait" : "pointer",
              opacity: syncing ? 0.6 : 1,
            }}
          >
            {syncing ? "Syncing..." : "Sync"}
          </button>
        </div>
      </div>

      <div style={{ maxHeight: 160, overflowY: "auto" }}>
        <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 10 }}>
          <thead>
            <tr style={{ color: theme.muted }}>
              <th style={{ textAlign: "left", padding: "2px 4px", fontWeight: 400 }}>Model</th>
              <th style={{ textAlign: "right", padding: "2px 4px", fontWeight: 400 }}>Input</th>
              <th style={{ textAlign: "right", padding: "2px 4px", fontWeight: 400 }}>Output</th>
              <th style={{ textAlign: "right", padding: "2px 4px", fontWeight: 400 }}>Cache</th>
              <th style={{ textAlign: "center", padding: "2px 4px", fontWeight: 400 }}>Src</th>
            </tr>
          </thead>
          <tbody>
            {allModels.map((model) => {
              const p = prices[model] || { input: 0, output: 0, cache: 0, source: "" };
              const isManual = p.source === "manual";
              return (
                <tr key={model} style={{ borderTop: `1px solid ${theme.border}` }}>
                  <td style={{ padding: "3px 4px", fontSize: 10, color: isManual ? theme.accentGreen : theme.foreground }}>
                    {model}
                  </td>
                  <td style={{ padding: "2px" }}>
                    <input
                      type="number"
                      step="0.01"
                      value={p.input || ""}
                      onChange={(e) => updatePrice(model, "input", parseFloat(e.target.value) || 0)}
                      style={cellStyle}
                    />
                  </td>
                  <td style={{ padding: "2px" }}>
                    <input
                      type="number"
                      step="0.01"
                      value={p.output || ""}
                      onChange={(e) => updatePrice(model, "output", parseFloat(e.target.value) || 0)}
                      style={cellStyle}
                    />
                  </td>
                  <td style={{ padding: "2px" }}>
                    <input
                      type="number"
                      step="0.01"
                      value={p.cache || ""}
                      onChange={(e) => updatePrice(model, "cache", parseFloat(e.target.value) || 0)}
                      style={cellStyle}
                    />
                  </td>
                  <td style={{ textAlign: "center", fontSize: 9, color: theme.muted }}>
                    {p.source || "—"}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Integrate PriceTable into Settings**

In `Settings.tsx`, import and render:

```tsx
import { PriceTable } from "./PriceTable";
```

Add after the Cost Model Whitelist section:

```tsx
{/* Model Prices */}
<PriceTable
  config={draft}
  models={models}
  onUpdate={(prices) => setDraft({ ...draft, cost: { ...draft.cost, model_prices: prices } })}
  onSyncComplete={(newConfig) => setDraft(newConfig)}
  theme={theme}
/>
```

The `models` prop needs to come from the parent. Update the Settings Props interface:

```tsx
interface Props {
  config: Config;
  models: string[];
  onSave: (config: Config) => void;
  onClose: () => void;
  theme: ThemeTokens;
}
```

- [ ] **Step 3: Pass models to Settings from App.tsx**

In `App.tsx`, pass the models list (already fetched via `useMonitorData`) to Settings:

```tsx
<Settings config={config} models={models} onSave={...} onClose={...} theme={theme} />
```

- [ ] **Step 4: Verify dev server**

Run: `pnpm dev`
Expected: Price table renders with model list, sync button works.

- [ ] **Step 5: Commit**

```bash
git add src/components/PriceTable.tsx src/components/Settings.tsx src/App.tsx
git commit -m "feat: add model price table with sync button"
```

---

### Task 11: Frontend — Data Management UI

**Files:**
- Create: `src/components/DataManagement.tsx`
- Modify: `src/components/Settings.tsx`

- [ ] **Step 1: Create DataManagement component**

```tsx
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ThemeTokens } from "../theme";

interface Props {
  models: string[];
  theme: ThemeTokens;
}

export function DataManagement({ models, theme }: Props) {
  const [selectedModel, setSelectedModel] = useState("");
  const [confirmAll, setConfirmAll] = useState(false);

  const deleteByModel = async () => {
    if (!selectedModel) return;
    if (!window.confirm(`Delete all data for "${selectedModel}"?`)) return;
    await invoke("delete_model_data", { model: selectedModel });
    setSelectedModel("");
  };

  const deleteAll = async () => {
    if (!confirmAll) {
      setConfirmAll(true);
      return;
    }
    await invoke("delete_all_data");
    setConfirmAll(false);
  };

  const selectStyle = {
    background: theme.card,
    border: `1px solid ${theme.border}`,
    borderRadius: 4,
    padding: "4px 8px",
    fontSize: 11,
    color: theme.foreground,
    outline: "none",
    flex: 1,
  };

  const dangerBtn = {
    background: "#EF4444",
    border: "none",
    borderRadius: 4,
    color: "#fff",
    fontSize: 10,
    padding: "4px 10px",
    cursor: "pointer",
  };

  return (
    <div>
      <div style={{ fontSize: 9, color: theme.muted, textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 6 }}>
        Data Management
      </div>

      <div style={{ display: "flex", gap: 6, alignItems: "center", marginBottom: 8 }}>
        <select value={selectedModel} onChange={(e) => setSelectedModel(e.target.value)} style={selectStyle}>
          <option value="">Select model...</option>
          {models.map((m) => <option key={m} value={m}>{m}</option>)}
        </select>
        <button onClick={deleteByModel} disabled={!selectedModel} style={{ ...dangerBtn, opacity: selectedModel ? 1 : 0.4 }}>
          Delete
        </button>
      </div>

      <button onClick={deleteAll} style={{ ...dangerBtn, background: confirmAll ? "#DC2626" : "#EF4444" }}>
        {confirmAll ? "Confirm Delete ALL Data" : "Delete All Data"}
      </button>
      {confirmAll && (
        <button
          onClick={() => setConfirmAll(false)}
          style={{ marginLeft: 8, background: "transparent", border: `1px solid ${theme.border}`, borderRadius: 4, padding: "4px 10px", fontSize: 10, color: theme.muted, cursor: "pointer" }}
        >
          Cancel
        </button>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Add DataManagement to Settings**

```tsx
import { DataManagement } from "./DataManagement";
```

Add at the bottom of the scrollable area, before the footer:

```tsx
{/* Data Management */}
<DataManagement models={models} theme={theme} />
```

- [ ] **Step 3: Verify dev server**

Run: `pnpm dev`
Expected: Data management section renders with model dropdown and delete buttons.

- [ ] **Step 4: Commit**

```bash
git add src/components/DataManagement.tsx src/components/Settings.tsx
git commit -m "feat: add data management UI with delete by model/all"
```

---

### Task 12: Integration Testing and Final Verification

**Files:**
- All modified files

- [ ] **Step 1: Full build check**

Run: `cd src-tauri && cargo build`
Expected: PASS — full binary compiles.

- [ ] **Step 2: Run the app**

Run: `pnpm tauri dev`
Expected: App launches, tray icon appears, existing metrics still work.

- [ ] **Step 3: Verify cost display**

1. Open Settings
2. Enable "$ Cost" in tray items
3. Set a model price manually (e.g., claude-opus-4-7: input=15, output=75, cache=1.88)
4. Save settings
5. Verify tray shows `$X.XX` value

- [ ] **Step 4: Verify price sync**

1. Open Settings → Price Table
2. Click "Sync" button
3. Verify prices populate from litellm
4. Verify manually set prices are not overwritten

- [ ] **Step 5: Verify data deletion**

1. Open Settings → Data Management
2. Select a model → Delete
3. Verify model data removed (check chart)
4. Test "Delete All" with confirmation

- [ ] **Step 6: Verify Codex monitoring**

1. If Codex CLI is installed, run a Codex session
2. Verify requests appear in the chart with source "codex"
3. Verify cost calculation includes Codex requests

- [ ] **Step 7: Final commit**

```bash
git add -A
git commit -m "feat: complete cost tracking + codex support integration"
```
