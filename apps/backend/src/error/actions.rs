use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use super::RepositoryError;
use super::api::ApiError;

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

impl From<ActionServiceError> for ApiError {
    fn from(e: ActionServiceError) -> Self {
        match e {
            ActionServiceError::Repo(re) => ApiError::from(re),
            ActionServiceError::ActionNotFound => {
                ApiError::new(StatusCode::NOT_FOUND, e.to_string())
            }
        }
    }
}

impl IntoResponse for ActionServiceError {
    fn into_response(self) -> Response {
        ApiError::from(self).into_response()
    }
}
