use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::codex_parser::CodexSessionTracker;
use crate::config::load_config;
use crate::parser::{ParsedRequest, SessionTracker};

pub fn start_polling(tx: mpsc::UnboundedSender<ParsedRequest>) {
    std::thread::spawn(move || {
        let claude_dir = dirs::home_dir().map(|h| h.join(".claude").join("projects"));
        let codex_dir = dirs::home_dir().map(|h| h.join(".codex").join("sessions"));

        let tracker = Arc::new(SessionTracker::new());
        let codex_tracker = Arc::new(CodexSessionTracker::new());
        let mut file_positions: HashMap<PathBuf, u64> = HashMap::new();

        let config = load_config();
        let watch_sources = config.cost.watch_sources.clone();

        if watch_sources.is_empty() || watch_sources.contains(&"claude".to_string()) {
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
        }

        if watch_sources.is_empty() || watch_sources.contains(&"codex".to_string()) {
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
        }

        loop {
            std::thread::sleep(Duration::from_millis(500));

            let config = load_config();
            let watch_sources = &config.cost.watch_sources;

            if watch_sources.is_empty() || watch_sources.contains(&"claude".to_string()) {
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
            }

            if watch_sources.is_empty() || watch_sources.contains(&"codex".to_string()) {
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

fn seed_last_user_timestamp(path: &Path, tracker: &Arc<SessionTracker>) {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let reader = BufReader::new(file);
    let mut last_user_line: Option<String> = None;
    for line in reader.lines() {
        if let Ok(l) = line {
            if l.contains("\"type\":\"user\"") || l.contains("\"type\": \"user\"") {
                last_user_line = Some(l);
            }
        }
    }
    if let Some(line) = last_user_line {
        tracker.parse_line(&line);
    }
}
