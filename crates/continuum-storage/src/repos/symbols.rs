use sqlx::SqlitePool;

use crate::error::StorageError;

pub struct SymbolRepo {
    pool: SqlitePool,
}

impl SymbolRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        run_id: &str,
        name: &str,
        kind: &str,
        file_path: &str,
        line_start: i32,
        line_end: i32,
        signature: Option<&str>,
    ) -> Result<i64, StorageError> {
        let result = sqlx::query(
            "INSERT INTO symbols (run_id, name, kind, file_path, line_start, line_end, signature) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(run_id)
        .bind(name)
        .bind(kind)
        .bind(file_path)
        .bind(line_start)
        .bind(line_end)
        .bind(signature)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get(&self, id: i64) -> Result<Option<SymbolRow>, StorageError> {
        let row = sqlx::query_as::<_, SymbolRow>(
            "SELECT id, run_id, name, kind, file_path, line_start, line_end, signature FROM symbols WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_by_run(&self, run_id: &str) -> Result<Vec<SymbolRow>, StorageError> {
        let rows = sqlx::query_as::<_, SymbolRow>(
            "SELECT id, run_id, name, kind, file_path, line_start, line_end, signature FROM symbols WHERE run_id = ? ORDER BY name ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SymbolRow {
    pub id: i64,
    pub run_id: String,
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub line_start: i32,
    pub line_end: i32,
    pub signature: Option<String>,
}
