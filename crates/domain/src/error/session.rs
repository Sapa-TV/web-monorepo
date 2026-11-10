use http::StatusCode;
use thiserror::Error;

use super::{RepositoryError, UserServiceError};

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
    #[error("account already belongs to another user")]
    AccountConflict,
    #[error("{0}")]
    Repo(RepositoryError),
    #[error("{0}")]
    User(UserServiceError),
}

impl From<RepositoryError> for SessionServiceError {
    fn from(e: RepositoryError) -> Self {
        SessionServiceError::Repo(e)
    }
}

impl From<UserServiceError> for SessionServiceError {
    fn from(e: UserServiceError) -> Self {
        SessionServiceError::User(e)
    }
}

impl SessionServiceError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            SessionServiceError::InvalidTicket
            | SessionServiceError::SessionNotFound
            | SessionServiceError::SessionExpired => StatusCode::UNAUTHORIZED,
            SessionServiceError::Exchange(_) => StatusCode::BAD_GATEWAY,
            SessionServiceError::AccountConflict => StatusCode::CONFLICT,
            SessionServiceError::Repo(RepositoryError::Conflict(_)) => StatusCode::CONFLICT,
            SessionServiceError::User(UserServiceError::Repo(RepositoryError::Conflict(_))) => {
                StatusCode::CONFLICT
            }
            SessionServiceError::Repo(_) | SessionServiceError::User(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}
