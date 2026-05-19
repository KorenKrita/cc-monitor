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
    pub project: String,
    pub source: String,
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
            "PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS requests (
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

        let _ = conn.execute("ALTER TABLE requests ADD COLUMN project TEXT DEFAULT ''", []);
        let _ = conn.execute("ALTER TABLE requests ADD COLUMN source TEXT DEFAULT 'claude'", []);
        let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_project ON requests(project)", []);
        let _ = conn.execute("CREATE INDEX IF NOT EXISTS idx_source ON requests(source)", []);

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

    pub fn query_requests(&self, since: &str, until: Option<&str>, model_filter: Option<&[String]>) -> Result<Vec<RequestRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut sql = "SELECT id, timestamp, model, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, duration_ms, project, source FROM requests WHERE timestamp >= ?1".to_string();

        let mut param_idx = 2;

        if let Some(_) = until {
            sql.push_str(&format!(" AND timestamp < ?{}", param_idx));
            param_idx += 1;
        }

        if let Some(models) = model_filter {
            if !models.is_empty() {
                let placeholders: Vec<String> = models.iter().enumerate().map(|(i, _)| format!("?{}", i + param_idx)).collect();
                sql.push_str(&format!(" AND model IN ({})", placeholders.join(",")));
            }
        }

        sql.push_str(" ORDER BY timestamp ASC");

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(since.to_string())];
        if let Some(u) = until {
            param_values.push(Box::new(u.to_string()));
        }
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
                project: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
                source: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
            })
        }).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

    pub fn get_latest(&self) -> Result<Option<RequestRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(
            "SELECT id, timestamp, model, input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, duration_ms, project, source FROM requests ORDER BY id DESC LIMIT 1"
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
                project: row.get::<_, Option<String>>(8)?.unwrap_or_default(),
                source: row.get::<_, Option<String>>(9)?.unwrap_or_default(),
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

    pub fn get_projects(&self) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT DISTINCT project FROM requests WHERE project != '' ORDER BY project")
            .map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0)).map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }

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
}
