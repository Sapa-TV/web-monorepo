use thiserror::Error;

use super::RepositoryError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ActionServiceError {
    #[error("action not found")]
    ActionNotFound,
    #[error("{0}")]
    Repo(RepositoryError),
}

impl From<RepositoryError> for ActionServiceError {
    fn from(e: RepositoryError) -> Self {
        ActionServiceError::Repo(e)
    }
}
