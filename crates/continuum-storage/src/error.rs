use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StorageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(String),
    #[error("vector error: {0}")]
    Vector(#[from] VectorError),
    #[error("not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum VectorError {
    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },
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
