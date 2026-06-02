use sqlx::SqlitePool;

use crate::error::StorageError;

/// Repository for storing indexed code symbols extracted from source files.
pub struct SymbolRepo {
    pool: SqlitePool,
}

impl SymbolRepo {
    /// Create a new `SymbolRepo` backed by the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a new symbol and return its auto-generated id.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
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

    /// Retrieve a symbol by its id.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn get(&self, id: i64) -> Result<Option<SymbolRow>, StorageError> {
        let row = sqlx::query_as::<_, SymbolRow>(
            "SELECT id, run_id, name, kind, file_path, line_start, line_end, signature FROM symbols WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// List all symbols for a run, ordered by name.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
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

/// A row representing an indexed code symbol.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SymbolRow {
    /// Auto-generated primary key.
    pub id: i64,
    /// The run this symbol was indexed in.
    pub run_id: String,
    /// The symbol name (e.g. function or struct name).
    pub name: String,
    /// The kind of symbol (e.g. "fn", "struct", "enum").
    pub kind: String,
    /// Path to the source file containing the symbol.
    pub file_path: String,
    /// The starting line of the symbol definition.
    pub line_start: i32,
    /// The ending line of the symbol definition.
    pub line_end: i32,
    /// Optional function signature or type annotation.
    pub signature: Option<String>,
}
