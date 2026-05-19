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

        // Record current end-of-file and pre-seed tracker with last user timestamps
        if let Ok(entries) = glob_jsonl_files(&claude_dir) {
            let mut positions = file_positions.lock().unwrap();
            for path in &entries {
                if let Ok(metadata) = std::fs::metadata(path) {
                    positions.insert(path.clone(), metadata.len());
                }
            }
            // Seed session tracker with recent user timestamps from each file
            for path in &entries {
                seed_last_user_timestamp(path, &tracker);
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

fn seed_last_user_timestamp(path: &Path, tracker: &Arc<SessionTracker>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };
    // Scan from the end to find the last "user" type entry
    for line in content.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }
        if trimmed.contains("\"type\":\"user\"") || trimmed.contains("\"type\": \"user\"") {
            // Feed it to the tracker to record the timestamp
            tracker.parse_line(trimmed);
            return;
        }
    }
}
