use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum RepositoryError {
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("database error: {0}")]
    Database(String),
}

impl From<sqlx::Error> for RepositoryError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}
