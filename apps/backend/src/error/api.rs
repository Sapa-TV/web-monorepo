use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use super::RepositoryError;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

pub struct ApiError {
    status: StatusCode,
    body: ErrorResponse,
}

impl ApiError {
    pub fn new(status: StatusCode, error: impl Into<String>) -> Self {
        Self {
            status,
            body: ErrorResponse {
                error: error.into(),
            },
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
                body: ErrorResponse { error: msg },
            },
            RepositoryError::Database(msg) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                body: ErrorResponse { error: msg },
            },
        }
    }
}