use thiserror::Error;

use super::ActionServiceError;
use super::RepositoryError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RuleServiceError {
    #[error("rule not found")]
    RuleNotFound,
    #[error("action not found")]
    ActionNotFound,
    #[error("pattern is required for matcher {0}")]
    MissingPattern(String),
    #[error("{0}")]
    Repo(RepositoryError),
}

impl From<RepositoryError> for RuleServiceError {
    fn from(e: RepositoryError) -> Self {
        RuleServiceError::Repo(e)
    }
}

impl From<ActionServiceError> for RuleServiceError {
    fn from(e: ActionServiceError) -> Self {
        match e {
            ActionServiceError::ActionNotFound => RuleServiceError::ActionNotFound,
            ActionServiceError::Repo(re) => RuleServiceError::Repo(re),
        }
    }
}
