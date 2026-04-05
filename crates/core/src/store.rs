use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::error::Error;
use crate::models::SearchResult;

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn new(data_dir: &str) -> Result<Self, Error> {
        let db_path = format!("{}/finder.db", data_dir);
        let mut conn = Connection::open(&db_path)?;

        let migrations = Migrations::new(vec![M::up(
            "CREATE TABLE IF NOT EXISTS search_results (
                id TEXT PRIMARY KEY,
                query TEXT NOT NULL,
                confidence REAL NOT NULL,
                bbox_x_min REAL NOT NULL,
                bbox_y_min REAL NOT NULL,
                bbox_x_max REAL NOT NULL,
                bbox_y_max REAL NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )]);
        migrations.to_latest(&mut conn)?;

        Ok(Self { conn })
    }

    pub fn save_result(
        &self,
        query: &str,
        confidence: f64,
        x_min: f64,
        y_min: f64,
        x_max: f64,
        y_max: f64,
    ) -> Result<SearchResult, Error> {
        let id = format!(
            "{:016x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        self.conn.execute(
            "INSERT INTO search_results (id, query, confidence, bbox_x_min, bbox_y_min, bbox_x_max, bbox_y_max) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![id, query, confidence, x_min, y_min, x_max, y_max],
        )?;
        self.get_result(&id)
    }

    pub fn get_result(&self, id: &str) -> Result<SearchResult, Error> {
        self.conn
            .query_row(
                "SELECT id, query, confidence, bbox_x_min, bbox_y_min, bbox_x_max, bbox_y_max, created_at FROM search_results WHERE id = ?1",
                rusqlite::params![id],
                |row| {
                    Ok(SearchResult {
                        id: row.get(0)?,
                        query: row.get(1)?,
                        confidence: row.get(2)?,
                        bbox_x_min: row.get(3)?,
                        bbox_y_min: row.get(4)?,
                        bbox_x_max: row.get(5)?,
                        bbox_y_max: row.get(6)?,
                        created_at: row.get(7)?,
                    })
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    Error::NotFound(format!("Result {}", id))
                }
                other => Error::Database(other),
            })
    }

    pub fn list_results(&self, limit: u32) -> Result<Vec<SearchResult>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, query, confidence, bbox_x_min, bbox_y_min, bbox_x_max, bbox_y_max, created_at FROM search_results ORDER BY created_at DESC LIMIT ?1"
        )?;
        let results = stmt
            .query_map(rusqlite::params![limit], |row| {
                Ok(SearchResult {
                    id: row.get(0)?,
                    query: row.get(1)?,
                    confidence: row.get(2)?,
                    bbox_x_min: row.get(3)?,
                    bbox_y_min: row.get(4)?,
                    bbox_x_max: row.get(5)?,
                    bbox_y_max: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_get_result() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().to_str().unwrap()).unwrap();

        let result = store
            .save_result("my keys", 0.92, 0.1, 0.2, 0.4, 0.5)
            .unwrap();
        assert_eq!(result.query, "my keys");
        assert!((result.confidence - 0.92).abs() < f64::EPSILON);

        let fetched = store.get_result(&result.id).unwrap();
        assert_eq!(fetched.id, result.id);
    }

    #[test]
    fn test_list_results() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().to_str().unwrap()).unwrap();

        store.save_result("keys", 0.9, 0.1, 0.2, 0.3, 0.4).unwrap();
        store.save_result("knife", 0.8, 0.5, 0.6, 0.7, 0.8).unwrap();

        let results = store.list_results(10).unwrap();
        assert_eq!(results.len(), 2);
    }
}
