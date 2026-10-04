use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use super::RepositoryError;
use super::api::ApiError;

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
}

impl From<RepositoryError> for SheetsError {
    fn from(e: RepositoryError) -> Self {
        SheetsError::Repo(e)
    }
}

impl From<SheetsError> for ApiError {
    fn from(e: SheetsError) -> Self {
        match e {
            SheetsError::Repo(re) => ApiError::from(re),
            e => ApiError::new(e.status_code(), e.to_string()),
        }
    }
}

impl SheetsError {
    fn status_code(&self) -> StatusCode {
        match self {
            SheetsError::NotConfigured
            | SheetsError::MissingSpreadsheetId
            | SheetsError::InvalidSpreadsheetUrl => StatusCode::BAD_REQUEST,
            SheetsError::Auth(_) | SheetsError::Api(_) => StatusCode::BAD_GATEWAY,
            SheetsError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SheetsError::Repo(RepositoryError::Conflict(_)) => StatusCode::CONFLICT,
            SheetsError::Repo(RepositoryError::Database(_)) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for SheetsError {
    fn into_response(self) -> Response {
        ApiError::from(self).into_response()
    }
}
