use axum::http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PlatformError {
    #[error("websocket error: {0}")]
    WebSocket(String),
    #[error("failed to parse eventsub frame: {0}")]
    Parse(String),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("eventsub subscription failed: {0}")]
    Subscription(String),
    #[error("connection dropped")]
    Disconnected,
    #[error("event sink closed")]
    SinkClosed,
    #[error("publish failed: {0}")]
    Publish(String),
    #[error("twitch api request failed: {0}")]
    TwitchApi(String),
}

impl From<PlatformError> for StatusCode {
    fn from(e: PlatformError) -> Self {
        match e {
            PlatformError::Auth(_) => StatusCode::UNAUTHORIZED,
            PlatformError::WebSocket(_)
            | PlatformError::Parse(_)
            | PlatformError::Subscription(_)
            | PlatformError::Disconnected
            | PlatformError::SinkClosed
            | PlatformError::Publish(_)
            | PlatformError::TwitchApi(_) => StatusCode::BAD_GATEWAY,
        }
    }
}
