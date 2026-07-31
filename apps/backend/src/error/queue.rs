use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use super::RepositoryError;
use super::api::ApiError;

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

impl From<QueueServiceError> for ApiError {
    fn from(e: QueueServiceError) -> Self {
        let status = match &e {
            QueueServiceError::Repo(_) => {
                return ApiError::from(match e {
                    QueueServiceError::Repo(re) => re,
                    _ => unreachable!(),
                });
            }
            QueueServiceError::NotFound | QueueServiceError::QueueEmpty => StatusCode::NOT_FOUND,
            QueueServiceError::AlreadyActive
            | QueueServiceError::NotSpinning
            | QueueServiceError::NotCancellable => StatusCode::CONFLICT,
            QueueServiceError::NoSlots
            | QueueServiceError::UserNotFound
            | QueueServiceError::RarityNotFound => StatusCode::UNPROCESSABLE_ENTITY,
        };
        ApiError::new(status, e.to_string())
    }
}

impl IntoResponse for QueueServiceError {
    fn into_response(self) -> Response {
        ApiError::from(self).into_response()
    }
}
