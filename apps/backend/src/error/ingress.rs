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
}
