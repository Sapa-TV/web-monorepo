use axum::http::StatusCode;

use super::api::ApiError;

pub use domain::error::user::*;

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
            _ => ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    }
}
