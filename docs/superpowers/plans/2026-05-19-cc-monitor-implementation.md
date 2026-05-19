# CC Monitor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a macOS menu bar app that monitors Claude Code token usage rates and TTFT in real-time, with per-model statistics and configurable display.

**Architecture:** Tauri app with Rust backend handling file watching, JSONL parsing, SQLite storage, and tray updates. React frontend renders an ECharts line chart in a popover panel with model filtering, metric switching, and time range selection. Communication via Tauri events (backend→frontend push) and Tauri commands (frontend→backend queries).

**Tech Stack:** Tauri 2.x, Rust (notify, rusqlite, serde_json), React 18, TailwindCSS 4, ECharts 5, TypeScript

---

## File Structure

```
cc-monitor/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # Tauri app setup, tray, window
│   │   ├── lib.rs               # Module declarations
│   │   ├── watcher.rs           # fs notify watcher for JSONL files
│   │   ├── parser.rs            # JSONL line parsing + TTFT calculation
│   │   ├── db.rs                # SQLite schema + queries
│   │   ├── tray.rs              # Tray text formatting + updates
│   │   ├── config.rs            # Settings load/save
│   │   └── commands.rs          # Tauri IPC commands for frontend
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── main.tsx                 # React entry
│   ├── App.tsx                  # Root layout + theme provider
│   ├── theme.ts                 # Theme tokens (dark/light)
│   ├── components/
│   │   ├── Chart.tsx            # ECharts line chart
│   │   ├── ModelFilter.tsx      # Left sidebar model checkboxes
│   │   ├── MetricTabs.tsx       # Out/In/TTFT switcher
│   │   └── TimeRangeTabs.tsx    # 1h/Today/Yesterday switcher
│   ├── hooks/
│   │   ├── useMonitorData.ts    # Listen to Tauri events, manage state
│   │   └── useSettings.ts      # Load/save settings via Tauri commands
│   └── types.ts                 # Shared TypeScript types
├── index.html
├── package.json
├── tailwind.config.ts
├── tsconfig.json
├── vite.config.ts
└── docs/
```

---

## Task 1: Project Scaffolding

**Files:**
- Create: `package.json`, `tsconfig.json`, `vite.config.ts`, `tailwind.config.ts`, `index.html`, `src/main.tsx`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Initialize Tauri project**

Run:
```bash
cd /Users/korenkrita/Coding/cc-monitor
npm create tauri-app@latest . -- --template react-ts --manager npm --yes
```

If the directory already has files, use `--force` or init manually. Expected: scaffolded Tauri + React + Vite project.

- [ ] **Step 2: Install frontend dependencies**

Run:
```bash
cd /Users/korenkrita/Coding/cc-monitor
npm install echarts echarts-for-react @tauri-apps/api @tauri-apps/plugin-shell
npm install -D tailwindcss @tailwindcss/vite
```

- [ ] **Step 3: Install Rust dependencies**

Edit `src-tauri/Cargo.toml` dependencies:
```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
notify = { version = "7", features = ["macos_fsevent"] }
rusqlite = { version = "0.32", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
dirs = "6"
tokio = { version = "1", features = ["sync", "fs"] }
```

- [ ] **Step 4: Configure TailwindCSS**

Create `src/index.css`:
```css
@import "tailwindcss";
```

Update `vite.config.ts`:
```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_"],
});
```

- [ ] **Step 5: Verify build**

Run:
```bash
cd /Users/korenkrita/Coding/cc-monitor
npm run tauri build -- --debug 2>&1 | tail -5
```

Expected: successful compilation (warnings OK).

- [ ] **Step 6: Commit**

```bash
git add -A && git -c user.name="KorenKrita" -c user.email="KorenKrita@gmail.com" commit -m "feat: scaffold Tauri + React + TailwindCSS project"
```

---

## Task 2: Rust — Config Module

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write config.rs**

```rust
// src-tauri/src/config.rs
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default)]
    pub tray: TrayConfig,
    #[serde(default)]
    pub model_aliases: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrayConfig {
    #[serde(default = "default_items")]
    pub items: Vec<String>,
    #[serde(default = "default_model_filter")]
    pub model_filter: String,
    #[serde(default)]
    pub model_whitelist: Vec<String>,
}

fn default_theme() -> String { "system".into() }
fn default_items() -> Vec<String> { vec!["out_rate".into(), "in_rate".into(), "ttft".into()] }
fn default_model_filter() -> String { "last".into() }

impl Default for TrayConfig {
    fn default() -> Self {
        Self {
            items: default_items(),
            model_filter: default_model_filter(),
            model_whitelist: vec![],
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self { theme: default_theme(), tray: TrayConfig::default(), model_aliases: std::collections::HashMap::new() }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("cc-monitor")
}

pub fn load_config() -> Config {
    let path = config_dir().join("settings.json");
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn save_config(config: &Config) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(dir.join("settings.json"), content).map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Register module in lib.rs**

```rust
// src-tauri/src/lib.rs
pub mod config;
pub mod db;
pub mod parser;
pub mod watcher;
pub mod tray;
pub mod commands;
```

- [ ] **Step 3: Verify compilation**

Run: `cd /Users/korenkrita/Coding/cc-monitor/src-tauri && cargo check`
Expected: compiles (other modules can be empty stubs for now)

- [ ] **Step 4: Commit**

```bash
git add -A && git -c user.name="KorenKrita" -c user.email="KorenKrita@gmail.com" commit -m "feat: add config module with settings load/save"
```

---

## Task 3: Rust — SQLite Database Module

**Files:**
- Create: `src-tauri/src/db.rs`

- [ ] **Step 1: Write db.rs**

```rust
// src-tauri/src/db.rs
use rusqlite::{Connection, params};
use serde::Serialize;
use std::sync::Mutex;

use crate::config::config_dir;

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
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new() -> Result<Self, String> {
        let dir = config_dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join("data.db");
        let conn = Connection::open(path).map_err(|e| e.to_string())?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS requests (
                id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                model TEXT NOT NULL,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                cache_creation_tokens INTEGER DEFAULT 0,
                cache_read_tokens INTEGER DEFAULT 0,
                duration_ms INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_timestamp ON requests(timestamp);
            CREATE INDEX IF NOT EXISTS idx_model ON requests(model);"
        ).map_err(|e| e.to_string())?;

        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn insert_request(
        &self,
        timestamp: &str,
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
        cache_creation_tokens: i64,
        cache_read_tokens: i64,
        duration_ms: Option<i64>,
    ) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO requests (timestamp, model, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, duration_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![timestamp, model, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, duration_ms],
        ).map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    pub fn query_requests(&self, since: &str, model_filter: Option<&[String]>) -> Result<Vec<RequestRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut sql = "SELECT id, timestamp, model, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, duration_ms FROM requests WHERE timestamp >= ?1".to_string();

        if let Some(models) = model_filter {
            if !models.is_empty() {
                let placeholders: Vec<String> = models.iter().enumerate().map(|(i, _)| format!("?{}", i + 2)).collect();
                sql.push_str(&format!(" AND model IN ({})", placeholders.join(",")));
            }
        }

        sql.push_str(" ORDER BY timestamp ASC");

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(since.to_string())];
        if let Some(models) = model_filter {
            for m in models {
                param_values.push(Box::new(m.clone()));
            }
        }

        let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

        let rows = stmt.query_map(params_ref.as_slice(), |row| {
            Ok(RequestRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                model: row.get(2)?,
                input_tokens: row.get(3)?,
                output_tokens: row.get(4)?,
                cache_creation_tokens: row.get(5)?,
                cache_read_tokens: row.get(6)?,
                duration_ms: row.get(7)?,
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_latest(&self) -> Result<Option<RequestRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, model, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, duration_ms FROM requests ORDER BY id DESC LIMIT 1"
        ).map_err(|e| e.to_string())?;

        let mut rows = stmt.query_map([], |row| {
            Ok(RequestRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                model: row.get(2)?,
                input_tokens: row.get(3)?,
                output_tokens: row.get(4)?,
                cache_creation_tokens: row.get(5)?,
                cache_read_tokens: row.get(6)?,
                duration_ms: row.get(7)?,
            })
        }).map_err(|e| e.to_string())?;

        match rows.next() {
            Some(Ok(record)) => Ok(Some(record)),
            Some(Err(e)) => Err(e.to_string()),
            None => Ok(None),
        }
    }

    pub fn get_models(&self) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT model, COUNT(*) as cnt FROM requests GROUP BY model ORDER BY cnt DESC").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd /Users/korenkrita/Coding/cc-monitor/src-tauri && cargo check`

- [ ] **Step 3: Commit**

```bash
git add -A && git -c user.name="KorenKrita" -c user.email="KorenKrita@gmail.com" commit -m "feat: add SQLite database module"
```

---

## Task 4: Rust — JSONL Parser

**Files:**
- Create: `src-tauri/src/parser.rs`

- [ ] **Step 1: Write parser.rs**

```rust
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

#[derive(Debug, Clone)]
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
```

- [ ] **Step 2: Verify compilation**

Run: `cd /Users/korenkrita/Coding/cc-monitor/src-tauri && cargo check`

- [ ] **Step 3: Commit**

```bash
git add -A && git -c user.name="KorenKrita" -c user.email="KorenKrita@gmail.com" commit -m "feat: add JSONL parser with session tracking"
```

---

## Task 5: Rust — File Watcher

**Files:**
- Create: `src-tauri/src/watcher.rs`

- [ ] **Step 1: Write watcher.rs**

```rust
// src-tauri/src/watcher.rs
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

use crate::parser::{ParsedRequest, SessionTracker};

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    pub fn start(
        tx: mpsc::UnboundedSender<ParsedRequest>,
    ) -> Result<Self, String> {
        let claude_dir = dirs::home_dir()
            .ok_or("no home dir")?
            .join(".claude")
            .join("projects");

        if !claude_dir.exists() {
            return Err(format!("{} does not exist", claude_dir.display()));
        }

        let tracker = Arc::new(SessionTracker::new());
        let file_positions: Arc<Mutex<HashMap<PathBuf, u64>>> = Arc::new(Mutex::new(HashMap::new()));

        // Record current end-of-file for all existing .jsonl files
        if let Ok(entries) = glob_jsonl_files(&claude_dir) {
            let mut positions = file_positions.lock().unwrap();
            for path in entries {
                if let Ok(metadata) = std::fs::metadata(&path) {
                    positions.insert(path, metadata.len());
                }
            }
        }

        let tracker_clone = tracker.clone();
        let positions_clone = file_positions.clone();
        let tx_clone = tx.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        for path in &event.paths {
                            if path.extension().map_or(false, |e| e == "jsonl") {
                                process_new_lines(
                                    path,
                                    &positions_clone,
                                    &tracker_clone,
                                    &tx_clone,
                                );
                            }
                        }
                    }
                }
            },
            Config::default(),
        ).map_err(|e| e.to_string())?;

        watcher.watch(&claude_dir, RecursiveMode::Recursive).map_err(|e| e.to_string())?;

        Ok(Self { _watcher: watcher })
    }
}

fn process_new_lines(
    path: &Path,
    positions: &Arc<Mutex<HashMap<PathBuf, u64>>>,
    tracker: &Arc<SessionTracker>,
    tx: &mpsc::UnboundedSender<ParsedRequest>,
) {
    let mut pos_map = match positions.lock() {
        Ok(m) => m,
        Err(_) => return,
    };

    let last_pos = pos_map.get(path).copied().unwrap_or(0);

    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return,
    };

    let metadata = match file.metadata() {
        Ok(m) => m,
        Err(_) => return,
    };

    let current_len = metadata.len();
    if current_len <= last_pos {
        return;
    }

    let mut reader = BufReader::new(file);
    if reader.seek(SeekFrom::Start(last_pos)).is_err() {
        return;
    }

    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    if let Some(request) = tracker.parse_line(trimmed) {
                        let _ = tx.send(request);
                    }
                }
            }
            Err(_) => break,
        }
    }

    pos_map.insert(path.to_path_buf(), current_len);
}

fn glob_jsonl_files(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut results = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Ok(mut sub) = glob_jsonl_files(&path) {
                    results.append(&mut sub);
                }
            } else if path.extension().map_or(false, |e| e == "jsonl") {
                results.push(path);
            }
        }
    }
    Ok(results)
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd /Users/korenkrita/Coding/cc-monitor/src-tauri && cargo check`

- [ ] **Step 3: Commit**

```bash
git add -A && git -c user.name="KorenKrita" -c user.email="KorenKrita@gmail.com" commit -m "feat: add file watcher for JSONL monitoring"
```

---

## Task 6: Rust — Tray Module

**Files:**
- Create: `src-tauri/src/tray.rs`

- [ ] **Step 1: Write tray.rs**

```rust
// src-tauri/src/tray.rs
use crate::config::TrayConfig;
use crate::parser::ParsedRequest;

pub fn format_tray_text(request: &ParsedRequest, config: &TrayConfig) -> String {
    let mut parts: Vec<String> = Vec::new();

    for item in &config.items {
        match item.as_str() {
            "out_rate" => parts.push(format!("↓{}", format_tokens(request.output_tokens))),
            "in_rate" => parts.push(format!("↑{}", format_tokens(request.input_tokens))),
            "ttft" => {
                if let Some(ms) = request.duration_ms {
                    parts.push(format_duration(ms));
                }
            }
            _ => {}
        }
    }

    if parts.is_empty() {
        "⬡".to_string()
    } else {
        format!("⬡ {}", parts.join(" "))
    }
}

fn format_tokens(tokens: i64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

fn format_duration(ms: i64) -> String {
    if ms >= 60_000 {
        format!("{:.1}m", ms as f64 / 60_000.0)
    } else if ms >= 1_000 {
        format!("{:.1}s", ms as f64 / 1_000.0)
    } else {
        format!("{}ms", ms)
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd /Users/korenkrita/Coding/cc-monitor/src-tauri && cargo check`

- [ ] **Step 3: Commit**

```bash
git add -A && git -c user.name="KorenKrita" -c user.email="KorenKrita@gmail.com" commit -m "feat: add tray text formatting"
```

---

## Task 7: Rust — Tauri Commands + Main Integration

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write commands.rs**

```rust
// src-tauri/src/commands.rs
use tauri::State;
use std::sync::Arc;

use crate::config::{Config, load_config, save_config};
use crate::db::{Database, RequestRecord};

pub struct AppState {
    pub db: Arc<Database>,
}

#[tauri::command]
pub fn get_requests(state: State<AppState>, since: String, models: Option<Vec<String>>) -> Result<Vec<RequestRecord>, String> {
    state.db.query_requests(&since, models.as_deref())
}

#[tauri::command]
pub fn get_latest(state: State<AppState>) -> Result<Option<RequestRecord>, String> {
    state.db.get_latest()
}

#[tauri::command]
pub fn get_models(state: State<AppState>) -> Result<Vec<String>, String> {
    state.db.get_models()
}

#[tauri::command]
pub fn get_config() -> Result<Config, String> {
    Ok(load_config())
}

#[tauri::command]
pub fn set_config(config: Config) -> Result<(), String> {
    save_config(&config)
}
```

- [ ] **Step 2: Write main.rs**

```rust
// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tauri::{
    tray::TrayIconBuilder, Manager, WindowEvent,
};
use tokio::sync::mpsc;

mod commands;
mod config;
mod db;
mod parser;
mod tray;
mod watcher;

use commands::AppState;

fn main() {
    let db = Arc::new(db::Database::new().expect("Failed to initialize database"));
    let config = config::load_config();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState { db: db.clone() })
        .setup(move |app| {
            let handle = app.handle().clone();
            let db_clone = db.clone();
            let config_clone = config.clone();

            // Create tray
            let _tray = TrayIconBuilder::new()
                .title("⬡")
                .tooltip("CC Monitor")
                .on_tray_icon_event(move |tray_icon, event| {
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
                        let app = tray_icon.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            // Start file watcher
            let (tx, mut rx) = mpsc::unbounded_channel();

            std::thread::spawn(move || {
                let _watcher = watcher::FileWatcher::start(tx)
                    .expect("Failed to start file watcher");
                std::thread::park();
            });

            // Process incoming requests
            let handle_clone = handle.clone();
            tauri::async_runtime::spawn(async move {
                while let Some(request) = rx.recv().await {
                    // Insert into DB
                    let _ = db_clone.insert_request(
                        &request.timestamp,
                        &request.model,
                        request.input_tokens,
                        request.output_tokens,
                        request.cache_creation_tokens,
                        request.cache_read_tokens,
                        request.duration_ms,
                    );

                    // Update tray text
                    let tray_text = tray::format_tray_text(&request, &config_clone.tray);
                    if let Some(tray) = handle_clone.tray_icon_by_id("main") {
                        let _ = tray.set_title(Some(&tray_text));
                    }

                    // Emit event to frontend
                    let _ = handle_clone.emit("new-request", &request);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_requests,
            commands::get_latest,
            commands::get_models,
            commands::get_config,
            commands::set_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Update tauri.conf.json for tray + popover window**

Key settings to add/modify in `src-tauri/tauri.conf.json`:
```json
{
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "CC Monitor",
        "width": 480,
        "height": 320,
        "visible": false,
        "resizable": false,
        "decorations": false,
        "alwaysOnTop": true,
        "skipTaskbar": true
      }
    ],
    "trayIcon": {
      "id": "main",
      "iconPath": "icons/icon.png"
    }
  }
}
```

- [ ] **Step 4: Verify compilation**

Run: `cd /Users/korenkrita/Coding/cc-monitor/src-tauri && cargo check`

- [ ] **Step 5: Commit**

```bash
git add -A && git -c user.name="KorenKrita" -c user.email="KorenKrita@gmail.com" commit -m "feat: integrate watcher, db, tray, and commands in main"
```

---

## Task 8: Frontend — Types + Theme System

**Files:**
- Create: `src/types.ts`, `src/theme.ts`

- [ ] **Step 1: Write types.ts**

```typescript
// src/types.ts
export interface RequestRecord {
  id: number;
  timestamp: string;
  model: string;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
  duration_ms: number | null;
}

export type Metric = "out_rate" | "in_rate" | "ttft";
export type TimeRange = "1h" | "today" | "yesterday";
export type Theme = "system" | "dark" | "light";

export interface Config {
  theme: Theme;
  tray: {
    items: Metric[];
    model_filter: "last" | "whitelist" | "all";
    model_whitelist: string[];
  };
  model_aliases: Record<string, string>;
}
```

- [ ] **Step 2: Write theme.ts**

```typescript
// src/theme.ts
export const darkTheme = {
  bg: "#0E1223",
  card: "#1A1E2F",
  border: "#272F42",
  muted: "#94A3B8",
  foreground: "#F8FAFC",
  mutedBg: "#272F42",
  gridLine: "#272F42",
  tabActiveBg: "#272F42",
  tabActiveText: "#F8FAFC",
  tabInactiveText: "#64748B",
  accentGreen: "#22C55E",
} as const;

export const lightTheme = {
  bg: "#FAFBFC",
  card: "#FFFFFF",
  border: "#E2E8F0",
  muted: "#94A3B8",
  foreground: "#1E293B",
  mutedBg: "#F1F5F9",
  gridLine: "#F1F5F9",
  tabActiveBg: "#FFFFFF",
  tabActiveText: "#1E293B",
  tabInactiveText: "#94A3B8",
  accentGreen: "#16A34A",
} as const;

// 10-color pool, assigned by model appearance order
export const colorPool = {
  dark: ["#6366f1", "#22C55E", "#F59E0B", "#EC4899", "#06B6D4", "#F97316", "#8B5CF6", "#14B8A6", "#EF4444", "#64748B"],
  light: ["#4F46E5", "#16A34A", "#D97706", "#DB2777", "#0891B2", "#EA580C", "#7C3AED", "#0D9488", "#DC2626", "#475569"],
} as const;

// Line style by color index: 0-2 solid, 3-5 dashed, 6+ dotted
export function getLineStyle(index: number): "solid" | "dashed" | [number, number] {
  if (index < 3) return "solid";
  if (index < 6) return "dashed";
  return [2, 3]; // dotted
}

export function getModelDisplayName(model: string, aliases: Record<string, string>): string {
  return aliases[model] || model;
}

export type ThemeTokens = typeof darkTheme;
```

- [ ] **Step 3: Commit**

```bash
git add -A && git -c user.name="KorenKrita" -c user.email="KorenKrita@gmail.com" commit -m "feat: add frontend types and theme tokens"
```

---

## Task 9: Frontend — App Shell + Layout

**Files:**
- Create: `src/App.tsx`, `src/hooks/useSettings.ts`, `src/hooks/useMonitorData.ts`
- Modify: `src/main.tsx`

- [ ] **Step 1: Write useSettings.ts**

```typescript
// src/hooks/useSettings.ts
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Config, Theme } from "../types";

export function useSettings() {
  const [config, setConfig] = useState<Config>({
    theme: "system",
    tray: { items: ["out_rate", "in_rate", "ttft"], model_filter: "last", model_whitelist: [] },
    model_aliases: {},
  });

  useEffect(() => {
    invoke<Config>("get_config").then(setConfig).catch(console.error);
  }, []);

  const resolvedTheme = (): "dark" | "light" => {
    if (config.theme === "system") {
      return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    }
    return config.theme;
  };

  return { config, setConfig, resolvedTheme };
}
```

- [ ] **Step 2: Write useMonitorData.ts**

```typescript
// src/hooks/useMonitorData.ts
import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { RequestRecord, Metric, TimeRange } from "../types";

export function useMonitorData() {
  const [requests, setRequests] = useState<RequestRecord[]>([]);
  const [models, setModels] = useState<string[]>([]);
  const [latest, setLatest] = useState<RequestRecord | null>(null);

  useEffect(() => {
    const unlisten = listen<RequestRecord>("new-request", (event) => {
      setRequests((prev) => [...prev, event.payload]);
      setLatest(event.payload);
      if (!models.includes(event.payload.model)) {
        setModels((prev) => [...new Set([...prev, event.payload.model])]);
      }
    });

    invoke<string[]>("get_models").then(setModels).catch(console.error);
    invoke<RequestRecord | null>("get_latest").then(setLatest).catch(console.error);

    return () => { unlisten.then((fn) => fn()); };
  }, []);

  const fetchData = useCallback(async (timeRange: TimeRange, modelFilter?: string[]) => {
    const since = getSinceTimestamp(timeRange);
    const data = await invoke<RequestRecord[]>("get_requests", {
      since,
      models: modelFilter && modelFilter.length > 0 ? modelFilter : null,
    });
    setRequests(data);
  }, []);

  return { requests, models, latest, fetchData };
}

function getSinceTimestamp(range: TimeRange): string {
  const now = new Date();
  switch (range) {
    case "1h":
      return new Date(now.getTime() - 60 * 60 * 1000).toISOString();
    case "today":
      return new Date(now.getFullYear(), now.getMonth(), now.getDate()).toISOString();
    case "yesterday": {
      const yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
      return yesterday.toISOString();
    }
  }
}
```

- [ ] **Step 3: Write App.tsx**

```typescript
// src/App.tsx
import { useState, useEffect } from "react";
import { useSettings } from "./hooks/useSettings";
import { useMonitorData } from "./hooks/useMonitorData";
import { darkTheme, lightTheme, ThemeTokens } from "./theme";
import { Metric, TimeRange } from "./types";
import { Chart } from "./components/Chart";
import { ModelFilter } from "./components/ModelFilter";
import { MetricTabs } from "./components/MetricTabs";
import { TimeRangeTabs } from "./components/TimeRangeTabs";

export default function App() {
  const { config, resolvedTheme } = useSettings();
  const { requests, models, latest, fetchData } = useMonitorData();
  const [metric, setMetric] = useState<Metric>("out_rate");
  const [timeRange, setTimeRange] = useState<TimeRange>("1h");
  const [selectedModels, setSelectedModels] = useState<string[]>([]);

  const theme: ThemeTokens = resolvedTheme() === "dark" ? darkTheme : lightTheme;
  const isDark = resolvedTheme() === "dark";

  useEffect(() => {
    fetchData(timeRange, selectedModels.length > 0 ? selectedModels : undefined);
  }, [timeRange, selectedModels, fetchData]);

  const latestValue = (): string => {
    if (!latest) return "—";
    switch (metric) {
      case "out_rate": return latest.output_tokens.toLocaleString();
      case "in_rate": return latest.input_tokens.toLocaleString();
      case "ttft": return latest.duration_ms ? `${(latest.duration_ms / 1000).toFixed(1)}s` : "—";
    }
  };

  const metricUnit = metric === "ttft" ? "sec" : "tok/req";

  return (
    <div style={{ background: theme.bg, color: theme.foreground, fontFamily: "'Fira Sans', system-ui, sans-serif", padding: 20, height: "100vh", overflow: "hidden" }}>
      {/* Header */}
      <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 14 }}>
        <MetricTabs value={metric} onChange={setMetric} theme={theme} />
        <TimeRangeTabs value={timeRange} onChange={setTimeRange} theme={theme} />
      </div>

      {/* Body */}
      <div style={{ display: "flex", gap: 14, height: "calc(100% - 50px)" }}>
        <ModelFilter
          models={models}
          selected={selectedModels}
          onChange={setSelectedModels}
          latestValue={latestValue()}
          metricUnit={metricUnit}
          theme={theme}
          isDark={isDark}
          aliases={config.model_aliases}
        />
        <Chart
          requests={requests}
          metric={metric}
          timeRange={timeRange}
          selectedModels={selectedModels}
          models={models}
          theme={theme}
          isDark={isDark}
          aliases={config.model_aliases}
        />
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Update main.tsx**

```typescript
// src/main.tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

- [ ] **Step 5: Commit**

```bash
git add -A && git -c user.name="KorenKrita" -c user.email="KorenKrita@gmail.com" commit -m "feat: add App shell with hooks and layout"
```

---

## Task 10: Frontend — MetricTabs + TimeRangeTabs Components

**Files:**
- Create: `src/components/MetricTabs.tsx`, `src/components/TimeRangeTabs.tsx`

- [ ] **Step 1: Write MetricTabs.tsx**

```typescript
// src/components/MetricTabs.tsx
import { Metric } from "../types";
import { ThemeTokens } from "../theme";

interface Props {
  value: Metric;
  onChange: (m: Metric) => void;
  theme: ThemeTokens;
}

const tabs: { key: Metric; label: string }[] = [
  { key: "out_rate", label: "Out" },
  { key: "in_rate", label: "In" },
  { key: "ttft", label: "TTFT" },
];

export function MetricTabs({ value, onChange, theme }: Props) {
  return (
    <div style={{ display: "flex", gap: 2, background: theme.mutedBg, borderRadius: 8, padding: 3 }}>
      {tabs.map((tab) => (
        <button
          key={tab.key}
          onClick={() => onChange(tab.key)}
          style={{
            padding: "4px 10px",
            borderRadius: 6,
            fontSize: 11,
            fontWeight: value === tab.key ? 500 : 400,
            background: value === tab.key ? theme.tabActiveBg : "transparent",
            color: value === tab.key ? theme.accentGreen : theme.tabInactiveText,
            border: "none",
            cursor: "pointer",
            boxShadow: value === tab.key ? "0 1px 2px rgba(0,0,0,0.06)" : "none",
          }}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
```

- [ ] **Step 2: Write TimeRangeTabs.tsx**

```typescript
// src/components/TimeRangeTabs.tsx
import { TimeRange } from "../types";
import { ThemeTokens } from "../theme";

interface Props {
  value: TimeRange;
  onChange: (t: TimeRange) => void;
  theme: ThemeTokens;
}

const tabs: { key: TimeRange; label: string }[] = [
  { key: "1h", label: "1h" },
  { key: "today", label: "Today" },
  { key: "yesterday", label: "Yest." },
];

export function TimeRangeTabs({ value, onChange, theme }: Props) {
  return (
    <div style={{ display: "flex", gap: 2, background: theme.mutedBg, borderRadius: 8, padding: 3 }}>
      {tabs.map((tab) => (
        <button
          key={tab.key}
          onClick={() => onChange(tab.key)}
          style={{
            padding: "4px 8px",
            borderRadius: 6,
            fontSize: 11,
            background: value === tab.key ? theme.tabActiveBg : "transparent",
            color: value === tab.key ? theme.tabActiveText : theme.tabInactiveText,
            border: "none",
            cursor: "pointer",
            boxShadow: value === tab.key ? "0 1px 2px rgba(0,0,0,0.06)" : "none",
          }}
        >
          {tab.label}
        </button>
      ))}
    </div>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add -A && git -c user.name="KorenKrita" -c user.email="KorenKrita@gmail.com" commit -m "feat: add MetricTabs and TimeRangeTabs components"
```

---

## Task 11: Frontend — ModelFilter Component

**Files:**
- Create: `src/components/ModelFilter.tsx`

- [ ] **Step 1: Write ModelFilter.tsx**

```typescript
// src/components/ModelFilter.tsx
import { ThemeTokens, colorPool, getModelDisplayName } from "../theme";

interface Props {
  models: string[];  // already sorted by usage count desc
  selected: string[];
  onChange: (models: string[]) => void;
  latestValue: string;
  metricUnit: string;
  theme: ThemeTokens;
  isDark: boolean;
  aliases: Record<string, string>;
}

export function ModelFilter({ models, selected, onChange, latestValue, metricUnit, theme, isDark, aliases }: Props) {
  const colors = isDark ? colorPool.dark : colorPool.light;

  const toggle = (model: string) => {
    if (selected.includes(model)) {
      onChange(selected.filter((m) => m !== model));
    } else {
      onChange([...selected, model]);
    }
  };

  const isSelected = (model: string) => selected.length === 0 || selected.includes(model);

  return (
    <div style={{ width: 100, paddingRight: 14, borderRight: `1px solid ${theme.border}`, display: "flex", flexDirection: "column" }}>
      <div style={{ fontSize: 9, color: theme.tabInactiveText, textTransform: "uppercase", letterSpacing: 0.5, marginBottom: 10 }}>
        Models
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {models.map((model, index) => {
          const color = colors[index % colors.length];
          return (
            <label
              key={model}
              onClick={() => toggle(model)}
              style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}
            >
              <div style={{
                width: 10, height: 10, borderRadius: 2,
                background: isSelected(model) ? color : "transparent",
                border: isSelected(model) ? "none" : `1.5px solid ${theme.border}`,
              }} />
              <span style={{
                fontSize: 10,
                color: isSelected(model) ? theme.foreground : theme.muted,
              }}>
                {getModelDisplayName(model, aliases)}
              </span>
            </label>
          );
        })}
      </div>

      {/* Latest value */}
      <div style={{ marginTop: "auto", paddingTop: 10, borderTop: `1px solid ${theme.border}` }}>
        <div style={{ fontSize: 8, color: theme.tabInactiveText, textTransform: "uppercase" }}>Latest</div>
        <div style={{ fontSize: 16, fontFamily: "'Fira Code', monospace", color: theme.accentGreen, fontWeight: 600, marginTop: 2 }}>
          {latestValue}
        </div>
        <div style={{ fontSize: 8, color: theme.tabInactiveText }}>{metricUnit}</div>
      </div>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add -A && git -c user.name="KorenKrita" -c user.email="KorenKrita@gmail.com" commit -m "feat: add ModelFilter component"
```

---

## Task 12: Frontend — ECharts Line Chart

**Files:**
- Create: `src/components/Chart.tsx`

- [ ] **Step 1: Write Chart.tsx**

```typescript
// src/components/Chart.tsx
import { useMemo } from "react";
import ReactECharts from "echarts-for-react";
import { RequestRecord, Metric, TimeRange } from "../types";
import { ThemeTokens, colorPool, getLineStyle, getModelDisplayName } from "../theme";

interface Props {
  requests: RequestRecord[];
  metric: Metric;
  timeRange: TimeRange;
  selectedModels: string[];
  models: string[];  // sorted by usage count, determines color assignment
  theme: ThemeTokens;
  isDark: boolean;
  aliases: Record<string, string>;
}

export function Chart({ requests, metric, timeRange, selectedModels, models, theme, isDark, aliases }: Props) {
  const colors = isDark ? colorPool.dark : colorPool.light;

  const option = useMemo(() => {
    const modelGroups = groupByModel(requests, selectedModels);
    const series = Object.entries(modelGroups).map(([model, records]) => {
      const colorIndex = models.indexOf(model);
      const color = colors[colorIndex % colors.length] || theme.muted;
      const lineType = getLineStyle(colorIndex >= 0 ? colorIndex : 0);
      return {
        name: getModelDisplayName(model, aliases),
        type: "line" as const,
        smooth: true,
        symbol: "none",
        lineStyle: {
          width: 2,
          type: lineType,
          color,
        },
        itemStyle: { color },
        data: records.map((r) => [r.timestamp, getValue(r, metric)]),
      };
    });

    return {
      animation: false,
      grid: { top: 12, right: 12, bottom: 28, left: 40 },
      xAxis: {
        type: "time",
        axisLine: { show: false },
        axisTick: { show: false },
        axisLabel: {
          fontSize: 9,
          color: theme.tabInactiveText,
          fontFamily: "'Fira Code', monospace",
          formatter: (value: number) => {
            const d = new Date(value);
            return `${d.getHours().toString().padStart(2, "0")}:${d.getMinutes().toString().padStart(2, "0")}`;
          },
        },
        splitLine: { show: false },
      },
      yAxis: {
        type: "value",
        axisLine: { show: false },
        axisTick: { show: false },
        axisLabel: {
          fontSize: 9,
          color: theme.tabInactiveText,
          fontFamily: "'Fira Code', monospace",
          formatter: (value: number) => {
            if (metric === "ttft") return `${(value / 1000).toFixed(1)}s`;
            if (value >= 1000) return `${(value / 1000).toFixed(1)}k`;
            return value.toString();
          },
        },
        splitLine: { lineStyle: { color: theme.gridLine, type: "solid" } },
      },
      tooltip: {
        trigger: "axis",
        backgroundColor: isDark ? "#272F42" : "#FFFFFF",
        borderColor: theme.border,
        textStyle: {
          color: theme.foreground,
          fontSize: 10,
          fontFamily: "'Fira Code', monospace",
        },
        axisPointer: {
          type: "line",
          lineStyle: { color: theme.border, type: "dashed" },
        },
        formatter: (params: any[]) => {
          if (!params.length) return "";
          const time = new Date(params[0].value[0]);
          const timeStr = `${time.getHours().toString().padStart(2, "0")}:${time.getMinutes().toString().padStart(2, "0")}`;
          let html = `<div style="margin-bottom:4px;color:${theme.muted}">${timeStr}</div>`;
          for (const p of params) {
            const val = metric === "ttft"
              ? `${(p.value[1] / 1000).toFixed(1)}s`
              : p.value[1].toLocaleString();
            html += `<div style="display:flex;align-items:center;gap:4px;margin-bottom:2px">`;
            html += `<span style="width:6px;height:6px;border-radius:50%;background:${p.color};display:inline-block"></span>`;
            html += `<span>${p.seriesName}: ${val}</span></div>`;
          }
          return html;
        },
      },
      series,
    };
  }, [requests, metric, selectedModels, models, theme, isDark, aliases]);

  return (
    <div style={{ flex: 1, background: theme.card, borderRadius: 8, padding: 8, overflow: "hidden" }}>
      <ReactECharts
        option={option}
        style={{ height: "100%", width: "100%" }}
        opts={{ renderer: "canvas" }}
        notMerge
      />
    </div>
  );
}

function groupByModel(requests: RequestRecord[], selectedModels: string[]): Record<string, RequestRecord[]> {
  const groups: Record<string, RequestRecord[]> = {};
  for (const r of requests) {
    if (selectedModels.length > 0 && !selectedModels.includes(r.model)) continue;
    if (!groups[r.model]) groups[r.model] = [];
    groups[r.model].push(r);
  }
  return groups;
}

function getValue(record: RequestRecord, metric: Metric): number {
  switch (metric) {
    case "out_rate": return record.output_tokens;
    case "in_rate": return record.input_tokens;
    case "ttft": return record.duration_ms ?? 0;
  }
}
```

- [ ] **Step 2: Verify frontend compiles**

Run: `cd /Users/korenkrita/Coding/cc-monitor && npm run build`

- [ ] **Step 3: Commit**

```bash
git add -A && git -c user.name="KorenKrita" -c user.email="KorenKrita@gmail.com" commit -m "feat: add ECharts line chart with tooltip and model colors"
```

---

## Task 13: Integration — Build and Test

**Files:**
- Modify: `src-tauri/tauri.conf.json` (final adjustments)

- [ ] **Step 1: Full build**

Run:
```bash
cd /Users/korenkrita/Coding/cc-monitor
npm run tauri build -- --debug
```

Expected: `.app` bundle created in `src-tauri/target/debug/bundle/macos/`

- [ ] **Step 2: Manual test**

1. Launch the app from the build output
2. Verify tray icon appears with `⬡`
3. Click tray icon → popover window shows
4. Open a Claude Code session in another terminal, send a message
5. Verify tray text updates with token counts
6. Verify chart shows new data point
7. Switch metric tabs (Out/In/TTFT) — chart Y-axis changes
8. Switch time range — chart X-axis adjusts
9. Toggle model checkboxes — lines appear/disappear

- [ ] **Step 3: Fix any issues found during testing**

- [ ] **Step 4: Final commit**

```bash
git add -A && git -c user.name="KorenKrita" -c user.email="KorenKrita@gmail.com" commit -m "feat: complete CC Monitor v0.1.0"
```

---

## Summary

| Task | Component | Estimated Time |
|------|-----------|---------------|
| 1 | Project scaffolding | 5 min |
| 2 | Config module | 3 min |
| 3 | SQLite database | 5 min |
| 4 | JSONL parser | 5 min |
| 5 | File watcher | 5 min |
| 6 | Tray formatting | 3 min |
| 7 | Main integration + commands | 5 min |
| 8 | Frontend types + theme | 3 min |
| 9 | App shell + hooks | 5 min |
| 10 | Tab components | 3 min |
| 11 | Model filter | 3 min |
| 12 | ECharts chart | 5 min |
| 13 | Build + test | 10 min |
| **Total** | | **~60 min** |
