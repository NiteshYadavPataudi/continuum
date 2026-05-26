mod error;
mod repos;
mod vector;

pub use error::{StorageError, VectorError};
pub use repos::*;
pub use sqlx::SqlitePool;
pub use vector::{memory::MemoryIndex, VectorBackend};

use sqlx::sqlite::SqlitePoolOptions;
use std::path::Path;

pub struct Storage {
    pool: SqlitePool,
    vector: VectorBackend,
}

impl Storage {
    pub async fn open(path: &Path) -> Result<Self, StorageError> {
        let database_url = format!("sqlite:{}", path.display());
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await?;

        sqlx::query("PRAGMA journal_mode=WAL;")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA foreign_keys=ON;")
            .execute(&pool)
            .await?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| StorageError::Migration(e.to_string()))?;

        Ok(Self {
            pool,
            vector: VectorBackend::Memory(MemoryIndex::new()),
        })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn vector(&self) -> &VectorBackend {
        &self.vector
    }
}
