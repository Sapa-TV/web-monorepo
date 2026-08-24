use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ActionError {
    #[error("capability not supported by this platform")]
    Unsupported,
    #[error("platform api error: {0}")]
    Api(String),
}
