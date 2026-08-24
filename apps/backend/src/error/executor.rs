use thiserror::Error;

use super::QueueServiceError;
use super::UserServiceError;
use super::platform_action::ActionError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ExecutorError {
    #[error("{0}")]
    User(#[from] UserServiceError),
    #[error("{0}")]
    Queue(#[from] QueueServiceError),
    #[error("platform action error: {0}")]
    Platform(#[from] ActionError),
}
