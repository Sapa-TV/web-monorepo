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

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl From<RepositoryError> for ApiError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::NotFound(id) => Self {
                status: StatusCode::NOT_FOUND,
                body: ErrorResponse {
                    error: format!("not found: id={id}"),
                },
            },
            RepositoryError::Database(msg) => Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                body: ErrorResponse { error: msg },
            },
        }
    }
}