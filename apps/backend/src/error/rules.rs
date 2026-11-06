use axum::http::StatusCode;

use super::api::ApiError;

pub use domain::error::rules::*;

impl From<RuleServiceError> for ApiError {
    fn from(e: RuleServiceError) -> Self {
        match e {
            RuleServiceError::Repo(re) => ApiError::from(re),
            RuleServiceError::RuleNotFound => ApiError::new(StatusCode::NOT_FOUND, e.to_string()),
            RuleServiceError::ActionNotFound | RuleServiceError::MissingPattern(_) => {
                ApiError::new(StatusCode::BAD_REQUEST, e.to_string())
            }
            _ => ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    }
}
