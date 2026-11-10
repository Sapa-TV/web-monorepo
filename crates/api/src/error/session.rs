use axum::http::StatusCode;

use super::api::ApiError;

pub use domain::error::session::*;

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
            SessionServiceError::Exchange(_) => {
                ApiError::new(StatusCode::BAD_GATEWAY, e.to_string())
            }
            SessionServiceError::AccountConflict => {
                ApiError::new(StatusCode::CONFLICT, e.to_string())
            }
            _ => ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    }
}
