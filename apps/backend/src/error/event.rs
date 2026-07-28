use thiserror::Error;

#[derive(Debug, Error)]
pub enum EventError {
    #[error("publish failed: {0}")]
    Publish(String),
}
