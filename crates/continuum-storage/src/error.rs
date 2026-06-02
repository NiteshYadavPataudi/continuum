use thiserror::Error;

/// Errors that can originate from storage operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StorageError {
    /// A SQLite error occurred (connection, query, etc.).
    #[error("sqlite error: {0}")]
    Sqlite(#[from] sqlx::Error),
    /// A database migration failed.
    #[error("migration error: {0}")]
    Migration(String),
    /// An error from the vector similarity backend.
    #[error("vector error: {0}")]
    Vector(#[from] VectorError),
    /// The requested resource was not found.
    #[error("not found: {0}")]
    NotFound(String),
}

/// Errors that can originate from vector index operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum VectorError {
    /// The provided vector dimension does not match the expected dimension.
    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },
    /// A generic error from the backing vector store.
    #[error("backing error: {0}")]
    Backing(String),
}

impl From<VectorError> for continuum_core::memory::VectorError {
    fn from(e: VectorError) -> Self {
        match e {
            VectorError::DimensionMismatch { expected, got } => Self::Dimension {
                expected: expected as u32,
                got: got as u32,
            },
            VectorError::Backing(msg) => Self::Backing(msg),
        }
    }
}
