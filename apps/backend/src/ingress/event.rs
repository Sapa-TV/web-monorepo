use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PlatformKind {
    Twitch,
    YouTube,
    VkVideoLive,
}

impl PlatformKind {
    pub const fn as_name(self) -> &'static str {
        match self {
            Self::Twitch => "twitch",
            Self::YouTube => "youtube",
            Self::VkVideoLive => "vk_video_live",
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PlatformEvent {
    pub platform: PlatformKind,
    pub sent_at: DateTime<Utc>,
    pub payload: PlatformEventPayload,
}

impl PlatformEvent {
    pub fn chat_message(
        platform: PlatformKind,
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
    fn platform_kind_as_name() {
        assert_eq!(PlatformKind::Twitch.as_name(), "twitch");
        assert_eq!(PlatformKind::YouTube.as_name(), "youtube");
        assert_eq!(PlatformKind::VkVideoLive.as_name(), "vk_video_live");
    }

    #[test]
    fn platform_kind_serde() {
        assert_eq!(
            serde_json::to_value(PlatformKind::Twitch).unwrap(),
            serde_json::json!("twitch")
        );
        assert_eq!(
            serde_json::to_value(PlatformKind::VkVideoLive).unwrap(),
            serde_json::json!("vk_video_live")
        );
    }

    #[test]
    fn chat_message_payload_type_name() {
        let event = PlatformEvent::chat_message(
            PlatformKind::Twitch,
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
