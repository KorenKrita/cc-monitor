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
        Self::init_schema(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        Self::init_schema(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn init_schema(conn: &Connection) -> Result<(), String> {
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

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::config::ModelPrice;

    fn setup_db() -> Database {
        Database::new_in_memory().unwrap()
    }

    #[test]
    fn test_insert_and_query() {
        let db = setup_db();
        db.insert_request("2026-05-20T10:00:00Z", "claude-opus-4-7", 1000, 500, 100, 200, Some(5000), "-Users-test", "claude").unwrap();

        let records = db.query_requests("2026-05-20T00:00:00Z", None, None).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].model, "claude-opus-4-7");
        assert_eq!(records[0].project, "-Users-test");
        assert_eq!(records[0].source, "claude");
        assert_eq!(records[0].input_tokens, 1000);
        assert_eq!(records[0].output_tokens, 500);
    }

    #[test]
    fn test_get_latest() {
        let db = setup_db();
        db.insert_request("2026-05-20T10:00:00Z", "model-a", 100, 50, 0, 0, None, "", "claude").unwrap();
        db.insert_request("2026-05-20T11:00:00Z", "model-b", 200, 100, 0, 0, None, "", "codex").unwrap();

        let latest = db.get_latest().unwrap().unwrap();
        assert_eq!(latest.model, "model-b");
        assert_eq!(latest.source, "codex");
    }

    #[test]
    fn test_get_models() {
        let db = setup_db();
        db.insert_request("2026-05-20T10:00:00Z", "model-a", 100, 50, 0, 0, None, "", "claude").unwrap();
        db.insert_request("2026-05-20T10:01:00Z", "model-a", 100, 50, 0, 0, None, "", "claude").unwrap();
        db.insert_request("2026-05-20T10:02:00Z", "model-b", 100, 50, 0, 0, None, "", "claude").unwrap();

        let models = db.get_models().unwrap();
        assert_eq!(models[0], "model-a"); // most frequent first
        assert_eq!(models[1], "model-b");
    }

    #[test]
    fn test_get_projects() {
        let db = setup_db();
        db.insert_request("2026-05-20T10:00:00Z", "m", 100, 50, 0, 0, None, "-Users-a", "claude").unwrap();
        db.insert_request("2026-05-20T10:01:00Z", "m", 100, 50, 0, 0, None, "-Users-b", "claude").unwrap();
        db.insert_request("2026-05-20T10:02:00Z", "m", 100, 50, 0, 0, None, "", "claude").unwrap();

        let projects = db.get_projects().unwrap();
        assert_eq!(projects.len(), 2);
        assert!(projects.contains(&"-Users-a".to_string()));
        assert!(projects.contains(&"-Users-b".to_string()));
    }

    #[test]
    fn test_delete_by_model() {
        let db = setup_db();
        db.insert_request("2026-05-20T10:00:00Z", "model-a", 100, 50, 0, 0, None, "", "claude").unwrap();
        db.insert_request("2026-05-20T10:01:00Z", "model-b", 100, 50, 0, 0, None, "", "claude").unwrap();

        let deleted = db.delete_by_model("model-a").unwrap();
        assert_eq!(deleted, 1);

        let models = db.get_models().unwrap();
        assert_eq!(models, vec!["model-b"]);
    }

    #[test]
    fn test_delete_all() {
        let db = setup_db();
        db.insert_request("2026-05-20T10:00:00Z", "m1", 100, 50, 0, 0, None, "", "claude").unwrap();
        db.insert_request("2026-05-20T10:01:00Z", "m2", 100, 50, 0, 0, None, "", "claude").unwrap();

        let deleted = db.delete_all().unwrap();
        assert_eq!(deleted, 2);

        let models = db.get_models().unwrap();
        assert!(models.is_empty());
    }

    #[test]
    fn test_calculate_cost_basic() {
        let db = setup_db();
        db.insert_request("2026-05-20T10:00:00Z", "claude-opus-4-7", 1_000_000, 500_000, 0, 100_000, None, "", "claude").unwrap();

        let mut prices = HashMap::new();
        prices.insert("claude-opus-4-7".to_string(), ModelPrice {
            input: 15.0,
            output: 75.0,
            cache: 1.88,
            source: "manual".to_string(),
        });

        let cost = db.calculate_cost("2026-05-20T00:00:00Z", &[], &[], &prices).unwrap();
        // 1M * 15 / 1M + 500K * 75 / 1M + 100K * 1.88 / 1M = 15 + 37.5 + 0.188 = 52.688
        assert!((cost - 52.688).abs() < 0.001);
    }

    #[test]
    fn test_calculate_cost_with_project_whitelist() {
        let db = setup_db();
        db.insert_request("2026-05-20T10:00:00Z", "m", 1_000_000, 0, 0, 0, None, "proj-a", "claude").unwrap();
        db.insert_request("2026-05-20T10:01:00Z", "m", 1_000_000, 0, 0, 0, None, "proj-b", "claude").unwrap();

        let mut prices = HashMap::new();
        prices.insert("m".to_string(), ModelPrice { input: 10.0, output: 0.0, cache: 0.0, source: "manual".to_string() });

        let cost = db.calculate_cost("2026-05-20T00:00:00Z", &["proj-a".to_string()], &[], &prices).unwrap();
        assert!((cost - 10.0).abs() < 0.001); // only proj-a counted
    }

    #[test]
    fn test_calculate_cost_no_price_returns_zero() {
        let db = setup_db();
        db.insert_request("2026-05-20T10:00:00Z", "unknown-model", 1_000_000, 500_000, 0, 0, None, "", "claude").unwrap();

        let prices = HashMap::new();
        let cost = db.calculate_cost("2026-05-20T00:00:00Z", &[], &[], &prices).unwrap();
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_calculate_cost_with_model_whitelist() {
        let db = setup_db();
        db.insert_request("2026-05-20T10:00:00Z", "m1", 1_000_000, 0, 0, 0, None, "", "claude").unwrap();
        db.insert_request("2026-05-20T10:01:00Z", "m2", 1_000_000, 0, 0, 0, None, "", "claude").unwrap();

        let mut prices = HashMap::new();
        prices.insert("m1".to_string(), ModelPrice { input: 10.0, output: 0.0, cache: 0.0, source: "manual".to_string() });
        prices.insert("m2".to_string(), ModelPrice { input: 20.0, output: 0.0, cache: 0.0, source: "manual".to_string() });

        let cost = db.calculate_cost("2026-05-20T00:00:00Z", &[], &["m1".to_string()], &prices).unwrap();
        assert!((cost - 10.0).abs() < 0.001);
    }
}
