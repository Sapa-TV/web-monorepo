use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use super::RepositoryError;
use super::api::ApiError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SessionServiceError {
    #[error("invalid or expired login ticket")]
    InvalidTicket,
    #[error("session not found")]
    SessionNotFound,
    #[error("session expired")]
    SessionExpired,
    #[error("{0}")]
    Repo(RepositoryError),
}

impl From<RepositoryError> for SessionServiceError {
    fn from(e: RepositoryError) -> Self {
        SessionServiceError::Repo(e)
    }
}

impl From<SessionServiceError> for ApiError {
    fn from(e: SessionServiceError) -> Self {
        match e {
            SessionServiceError::Repo(re) => ApiError::from(re),
            SessionServiceError::InvalidTicket => {
                ApiError::new(StatusCode::BAD_REQUEST, e.to_string())
            }
            SessionServiceError::SessionNotFound | SessionServiceError::SessionExpired => {
                ApiError::new(StatusCode::UNAUTHORIZED, e.to_string())
            }
        }
    }
}

impl IntoResponse for SessionServiceError {
    fn into_response(self) -> Response {
        ApiError::from(self).into_response()
    }
}
