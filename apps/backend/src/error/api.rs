use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

use super::RepositoryError;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}

#[non_exhaustive]
pub struct ApiError {
    status: StatusCode,
    body: ErrorResponse,
}

impl ApiError {
    pub fn new(status: StatusCode, error: impl Into<String>) -> Self {
        Self {
            status,
            body: ErrorResponse::new(error),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl From<RepositoryError> for ApiError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::Conflict(msg) => Self {
                status: StatusCode::CONFLICT,
                body: ErrorResponse::new(msg),
            },
            RepositoryError::Database(msg) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                body: ErrorResponse::new(msg),
            },
        }
    }
}
