use http::StatusCode;
use thiserror::Error;

use super::RepositoryError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PushServiceError {
    #[error("invalid subscription: {0}")]
    InvalidSubscription(String),
    #[error("{0}")]
    Repo(RepositoryError),
}

impl From<RepositoryError> for PushServiceError {
    fn from(e: RepositoryError) -> Self {
        PushServiceError::Repo(e)
    }
}

impl PushServiceError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            PushServiceError::InvalidSubscription(_) => StatusCode::BAD_REQUEST,
            PushServiceError::Repo(RepositoryError::Conflict(_)) => StatusCode::CONFLICT,
            PushServiceError::Repo(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
