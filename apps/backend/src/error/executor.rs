use thiserror::Error;

use super::QueueServiceError;
use super::UserServiceError;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ExecutorError {
    #[error("{0}")]
    User(#[from] UserServiceError),
    #[error("{0}")]
    Queue(#[from] QueueServiceError),
    #[error("twitch chat error: {0}")]
    Chat(String),
}
