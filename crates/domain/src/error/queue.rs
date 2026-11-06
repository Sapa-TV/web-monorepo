use http::StatusCode;
use thiserror::Error;

use super::RepositoryError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum QueueServiceError {
    #[error("queue entry not found")]
    NotFound,
    #[error("no pending or error entries")]
    QueueEmpty,
    #[error("a spin or error is already active")]
    AlreadyActive,
    #[error("entry is not in spinning state")]
    NotSpinning,
    #[error("only pending or error entries can be cancelled")]
    NotCancellable,
    #[error("no roulette slots configured")]
    NoSlots,
    #[error("user not found")]
    #[allow(dead_code)]
    UserNotFound,
    #[error("rarity not found")]
    RarityNotFound,
    #[error("{0}")]
    Repo(RepositoryError),
}

impl From<RepositoryError> for QueueServiceError {
    fn from(e: RepositoryError) -> Self {
        QueueServiceError::Repo(e)
    }
}

impl QueueServiceError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            QueueServiceError::NotFound | QueueServiceError::QueueEmpty => StatusCode::NOT_FOUND,
            QueueServiceError::AlreadyActive
            | QueueServiceError::NotSpinning
            | QueueServiceError::NotCancellable => StatusCode::CONFLICT,
            QueueServiceError::NoSlots
            | QueueServiceError::UserNotFound
            | QueueServiceError::RarityNotFound => StatusCode::UNPROCESSABLE_ENTITY,
            QueueServiceError::Repo(RepositoryError::Conflict(_)) => StatusCode::CONFLICT,
            QueueServiceError::Repo(RepositoryError::Database(_)) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            QueueServiceError::Repo(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
