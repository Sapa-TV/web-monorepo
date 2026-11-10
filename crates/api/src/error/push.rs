use axum::http::StatusCode;

use super::api::ApiError;

pub use domain::error::push::*;

impl From<PushServiceError> for ApiError {
    fn from(e: PushServiceError) -> Self {
        match e {
            PushServiceError::Repo(re) => ApiError::from(re),
            PushServiceError::InvalidSubscription(_) => {
                ApiError::new(StatusCode::BAD_REQUEST, e.to_string())
            }
            _ => ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    }
}
