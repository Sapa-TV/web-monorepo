use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use super::ActionServiceError;
use super::RepositoryError;
use super::api::ApiError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RuleServiceError {
    #[error("rule not found")]
    RuleNotFound,
    #[error("action not found")]
    ActionNotFound,
    #[error("pattern is required for matcher {0}")]
    MissingPattern(String),
    #[error("{0}")]
    Repo(RepositoryError),
}

impl From<RepositoryError> for RuleServiceError {
    fn from(e: RepositoryError) -> Self {
        RuleServiceError::Repo(e)
    }
}

impl From<ActionServiceError> for RuleServiceError {
    fn from(e: ActionServiceError) -> Self {
        match e {
            ActionServiceError::ActionNotFound => RuleServiceError::ActionNotFound,
            ActionServiceError::Repo(re) => RuleServiceError::Repo(re),
        }
    }
}

impl From<RuleServiceError> for ApiError {
    fn from(e: RuleServiceError) -> Self {
        match e {
            RuleServiceError::Repo(re) => ApiError::from(re),
            RuleServiceError::RuleNotFound => ApiError::new(StatusCode::NOT_FOUND, e.to_string()),
            RuleServiceError::ActionNotFound | RuleServiceError::MissingPattern(_) => {
                ApiError::new(StatusCode::BAD_REQUEST, e.to_string())
            }
        }
    }
}

impl IntoResponse for RuleServiceError {
    fn into_response(self) -> Response {
        ApiError::from(self).into_response()
    }
}
