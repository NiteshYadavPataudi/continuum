use sqlx::SqlitePool;

use crate::error::StorageError;

pub struct DecisionRepo {
    pool: SqlitePool,
}

impl DecisionRepo {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

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

    pub async fn get(&self, id: i64) -> Result<Option<DecisionRow>, StorageError> {
        let row = sqlx::query_as::<_, DecisionRow>(
            "SELECT id, run_id, agent_kind, reasoning, outcome, created_at FROM decisions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DecisionRow {
    pub id: i64,
    pub run_id: String,
    pub agent_kind: String,
    pub reasoning: String,
    pub outcome: String,
    pub created_at: String,
}
