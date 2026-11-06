use http::StatusCode;
use thiserror::Error;

use super::OrdersServiceError;
use super::RepositoryError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SheetsError {
    #[error("google sheets is not configured")]
    NotConfigured,
    #[error("spreadsheet id is not set; run an import first or set GOOGLE_SPREADSHEET_ID")]
    MissingSpreadsheetId,
    #[error("invalid spreadsheet url or id")]
    InvalidSpreadsheetUrl,
    #[error("google auth failed: {0}")]
    Auth(String),
    #[error("google sheets api error: {0}")]
    Api(String),
    #[error("config error: {0}")]
    Config(String),
    #[error("{0}")]
    Repo(RepositoryError),
    #[error("{0}")]
    Service(OrdersServiceError),
}

impl From<RepositoryError> for SheetsError {
    fn from(e: RepositoryError) -> Self {
        SheetsError::Repo(e)
    }
}

impl From<OrdersServiceError> for SheetsError {
    fn from(e: OrdersServiceError) -> Self {
        SheetsError::Service(e)
    }
}

impl SheetsError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            SheetsError::NotConfigured
            | SheetsError::MissingSpreadsheetId
            | SheetsError::InvalidSpreadsheetUrl => StatusCode::BAD_REQUEST,
            SheetsError::Auth(_) | SheetsError::Api(_) => StatusCode::BAD_GATEWAY,
            SheetsError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SheetsError::Repo(RepositoryError::Conflict(_)) => StatusCode::CONFLICT,
            SheetsError::Repo(RepositoryError::Database(_)) => StatusCode::INTERNAL_SERVER_ERROR,
            SheetsError::Repo(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SheetsError::Service(OrdersServiceError::NotFound) => StatusCode::NOT_FOUND,
            SheetsError::Service(OrdersServiceError::Invalid(_)) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            SheetsError::Service(OrdersServiceError::Repo(RepositoryError::Conflict(_))) => {
                StatusCode::CONFLICT
            }
            SheetsError::Service(OrdersServiceError::Repo(_)) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
