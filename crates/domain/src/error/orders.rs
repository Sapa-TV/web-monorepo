use http::StatusCode;
use thiserror::Error;

use super::RepositoryError;

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

impl OrdersServiceError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            OrdersServiceError::NotFound => StatusCode::NOT_FOUND,
            OrdersServiceError::Invalid(_) => StatusCode::UNPROCESSABLE_ENTITY,
            OrdersServiceError::Repo(RepositoryError::Conflict(_)) => StatusCode::CONFLICT,
            OrdersServiceError::Repo(RepositoryError::Database(_)) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            OrdersServiceError::Repo(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
