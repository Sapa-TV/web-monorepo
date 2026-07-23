use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum RepositoryError {
    #[error("entity not found: id={0}")]
    NotFound(u32),
    #[error("database error: {0}")]
    Database(String),
}