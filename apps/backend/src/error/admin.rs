use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use super::RepositoryError;
use super::api::ApiError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AdminServiceError {
    #[error("admin not found")]
    AdminNotFound,
    #[error("admin already exists")]
    AlreadyAdmin,
    #[error("cannot remove the last root admin")]
    CannotRemoveLastRoot,
    #[error("{0}")]
    Repo(RepositoryError),
}

impl From<RepositoryError> for AdminServiceError {
    fn from(e: RepositoryError) -> Self {
        AdminServiceError::Repo(e)
    }
}

impl From<AdminServiceError> for ApiError {
    fn from(e: AdminServiceError) -> Self {
        match e {
            AdminServiceError::Repo(re) => ApiError::from(re),
            AdminServiceError::AdminNotFound => ApiError::new(StatusCode::NOT_FOUND, e.to_string()),
            AdminServiceError::AlreadyAdmin => ApiError::new(StatusCode::CONFLICT, e.to_string()),
            AdminServiceError::CannotRemoveLastRoot => {
                ApiError::new(StatusCode::FORBIDDEN, e.to_string())
            }
        }
    }
}

impl IntoResponse for AdminServiceError {
    fn into_response(self) -> Response {
        ApiError::from(self).into_response()
    }
}
