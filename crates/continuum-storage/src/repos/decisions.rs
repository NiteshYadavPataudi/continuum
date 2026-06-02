use sqlx::SqlitePool;

use crate::error::StorageError;

/// Repository for recording and querying agent decisions.
pub struct DecisionRepo {
    pool: SqlitePool,
}

impl DecisionRepo {
    /// Create a new `DecisionRepo` backed by the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a new decision and return its auto-generated id.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn create(
        &self,
        run_id: &str,
        agent_kind: &str,
        reasoning: &str,
        outcome: &str,
    ) -> Result<i64, StorageError> {
        let result = sqlx::query(
            "INSERT INTO decisions (run_id, agent_kind, reasoning, outcome) VALUES (?, ?, ?, ?)",
        )
        .bind(run_id)
        .bind(agent_kind)
        .bind(reasoning)
        .bind(outcome)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    /// Retrieve a decision by its id.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn get(&self, id: i64) -> Result<Option<DecisionRow>, StorageError> {
        let row = sqlx::query_as::<_, DecisionRow>(
            "SELECT id, run_id, agent_kind, reasoning, outcome, created_at FROM decisions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// List all decisions for a given run, ordered by creation time.
    ///
    /// # Errors
    /// Returns `StorageError::Sqlite` on query failure.
    pub async fn list_by_run(&self, run_id: &str) -> Result<Vec<DecisionRow>, StorageError> {
        let rows = sqlx::query_as::<_, DecisionRow>(
            "SELECT id, run_id, agent_kind, reasoning, outcome, created_at FROM decisions WHERE run_id = ? ORDER BY created_at ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

/// A row representing a single agent decision.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DecisionRow {
    /// Auto-generated primary key.
    pub id: i64,
    /// The run this decision was made in.
    pub run_id: String,
    /// The kind of agent that made the decision (e.g. "planner", "coder").
    pub agent_kind: String,
    /// The reasoning behind the decision.
    pub reasoning: String,
    /// The outcome or action taken.
    pub outcome: String,
    /// ISO-8601 timestamp of when the decision was recorded.
    pub created_at: String,
}
