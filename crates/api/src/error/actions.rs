use axum::http::StatusCode;

use super::api::ApiError;

pub use domain::error::actions::*;

impl From<ActionServiceError> for ApiError {
    fn from(e: ActionServiceError) -> Self {
        match e {
            ActionServiceError::Repo(re) => ApiError::from(re),
            ActionServiceError::ActionNotFound => {
                ApiError::new(StatusCode::NOT_FOUND, e.to_string())
            }
            _ => ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    }
}
