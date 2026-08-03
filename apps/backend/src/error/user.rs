use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use super::RepositoryError;
use super::api::ApiError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum UserServiceError {
    #[error("user not found")]
    UserNotFound,
    #[error("platform link not found")]
    PlatformLinkNotFound,
    #[error("unknown platform: {0}")]
    UnknownPlatform(String),
    #[error("{0}")]
    Repo(RepositoryError),
}

impl From<RepositoryError> for UserServiceError {
    fn from(e: RepositoryError) -> Self {
        UserServiceError::Repo(e)
    }
}

impl From<UserServiceError> for ApiError {
    fn from(e: UserServiceError) -> Self {
        match e {
            UserServiceError::Repo(re) => ApiError::from(re),
            UserServiceError::UserNotFound | UserServiceError::PlatformLinkNotFound => {
                ApiError::new(StatusCode::NOT_FOUND, e.to_string())
            }
            UserServiceError::UnknownPlatform(_) => {
                ApiError::new(StatusCode::BAD_REQUEST, e.to_string())
            }
        }
    }
}

impl IntoResponse for UserServiceError {
    fn into_response(self) -> Response {
        ApiError::from(self).into_response()
    }
}
