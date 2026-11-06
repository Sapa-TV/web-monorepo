use thiserror::Error;

use super::RepositoryError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AdminServiceError {
    #[error("admin not found")]
    AdminNotFound,
    #[error("admin already exists")]
    AlreadyAdmin,
    #[error("cannot remove the last root admin")]
    CannotRemoveLastRoot,
    #[error("cannot remove your own account")]
    CannotRemoveSelf,
    #[error("{0}")]
    Repo(RepositoryError),
}

impl From<RepositoryError> for AdminServiceError {
    fn from(e: RepositoryError) -> Self {
        AdminServiceError::Repo(e)
    }
}
