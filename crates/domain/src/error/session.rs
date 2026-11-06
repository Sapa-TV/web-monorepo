use http::StatusCode;
use thiserror::Error;

use super::RepositoryError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SessionServiceError {
    #[error("invalid or expired login ticket")]
    InvalidTicket,
    #[error("session not found")]
    SessionNotFound,
    #[error("session expired")]
    SessionExpired,
    #[error("oauth code exchange failed: {0}")]
    Exchange(String),
    #[error("{0}")]
    Repo(RepositoryError),
}

impl From<RepositoryError> for SessionServiceError {
    fn from(e: RepositoryError) -> Self {
        SessionServiceError::Repo(e)
    }
}

impl SessionServiceError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            SessionServiceError::InvalidTicket
            | SessionServiceError::SessionNotFound
            | SessionServiceError::SessionExpired => StatusCode::UNAUTHORIZED,
            SessionServiceError::Exchange(_) => StatusCode::BAD_GATEWAY,
            SessionServiceError::Repo(RepositoryError::Conflict(_)) => StatusCode::CONFLICT,
            SessionServiceError::Repo(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
