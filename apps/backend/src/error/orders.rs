use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use super::RepositoryError;
use super::api::ApiError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OrdersServiceError {
    #[error("order not found")]
    NotFound,
    #[error("invalid order: {0}")]
    Invalid(String),
    #[error("{0}")]
    Repo(RepositoryError),
}

impl From<RepositoryError> for OrdersServiceError {
    fn from(e: RepositoryError) -> Self {
        OrdersServiceError::Repo(e)
    }
}

impl From<OrdersServiceError> for ApiError {
    fn from(e: OrdersServiceError) -> Self {
        match e {
            OrdersServiceError::Repo(re) => ApiError::from(re),
            e => ApiError::new(e.status_code(), e.to_string()),
        }
    }
}

impl OrdersServiceError {
    fn status_code(&self) -> StatusCode {
        match self {
            OrdersServiceError::NotFound => StatusCode::NOT_FOUND,
            OrdersServiceError::Invalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
            OrdersServiceError::Repo(RepositoryError::Conflict(_)) => StatusCode::CONFLICT,
            OrdersServiceError::Repo(RepositoryError::Database(_)) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

impl IntoResponse for OrdersServiceError {
    fn into_response(self) -> Response {
        ApiError::from(self).into_response()
    }
}
