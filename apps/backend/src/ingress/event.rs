use chrono::{DateTime, Utc};

use crate::platform::PlatformId;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PlatformEvent {
    pub platform: PlatformId,
    pub sent_at: DateTime<Utc>,
    pub payload: PlatformEventPayload,
}

impl PlatformEvent {
    pub fn chat_message(
        platform: PlatformId,
        user_id: String,
        user_name: String,
        text: String,
    ) -> Self {
        Self {
            platform,
            sent_at: Utc::now(),
            payload: PlatformEventPayload::ChatMessage(ChatMessage {
                user_id,
                user_name,
                text,
            }),
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PlatformEventPayload {
    ChatMessage(ChatMessage),
}

impl PlatformEventPayload {
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::ChatMessage(_) => "chat_message",
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ChatMessage {
    pub user_id: String,
    pub user_name: String,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_message_payload_type_name() {
        let event = PlatformEvent::chat_message(
            PlatformId::TWITCH,
            "1".to_string(),
            "viewer".to_string(),
            "hello".to_string(),
        );
        assert_eq!(event.payload.type_name(), "chat_message");
        match &event.payload {
            PlatformEventPayload::ChatMessage(msg) => assert_eq!(msg.text, "hello"),
        }
    }
}
