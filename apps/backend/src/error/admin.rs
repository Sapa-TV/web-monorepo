use axum::http::StatusCode;

use super::api::ApiError;

pub use domain::error::admin::*;

impl From<AdminServiceError> for ApiError {
    fn from(e: AdminServiceError) -> Self {
        match e {
            AdminServiceError::Repo(re) => ApiError::from(re),
            AdminServiceError::AdminNotFound => ApiError::new(StatusCode::NOT_FOUND, e.to_string()),
            AdminServiceError::AlreadyAdmin => ApiError::new(StatusCode::CONFLICT, e.to_string()),
            AdminServiceError::CannotRemoveLastRoot => {
                ApiError::new(StatusCode::FORBIDDEN, e.to_string())
            }
            AdminServiceError::CannotRemoveSelf => {
                ApiError::new(StatusCode::CONFLICT, e.to_string())
            }
            _ => ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    }
}
