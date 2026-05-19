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
