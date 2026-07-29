use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EventError {
    #[error("publish failed: {0}")]
    Publish(String),
}
