use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum RepositoryError {
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("database error: {0}")]
    Database(String),
}
