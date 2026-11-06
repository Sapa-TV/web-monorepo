use thiserror::Error;

use super::RepositoryError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum UserServiceError {
    #[error("user not found")]
    UserNotFound,
    #[error("platform link not found")]
    PlatformLinkNotFound,
    #[error("unknown platform: {0}")]
    UnknownPlatform(String),
    #[error("{0}")]
    Repo(RepositoryError),
}

impl From<RepositoryError> for UserServiceError {
    fn from(e: RepositoryError) -> Self {
        UserServiceError::Repo(e)
    }
}
