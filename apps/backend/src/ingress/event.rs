use chrono::{DateTime, Utc};

use crate::platform::PlatformId;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PlatformEvent {
    pub platform: PlatformId,
    pub event_id: String,
    pub sent_at: DateTime<Utc>,
    pub payload: PlatformEventPayload,
}

impl PlatformEvent {
    pub fn chat_message(
        platform: PlatformId,
        event_id: impl Into<String>,
        user_id: String,
        user_name: String,
        text: String,
    ) -> Self {
        Self {
            platform,
            event_id: event_id.into(),
            sent_at: Utc::now(),
            payload: PlatformEventPayload::ChatMessage(ChatMessage {
                user_id,
                user_name,
                text,
            }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn reward_redemption(
        platform: PlatformId,
        event_id: impl Into<String>,
        user_id: String,
        user_name: String,
        reward_id: String,
        reward_title: String,
        reward_cost: i64,
        user_input: String,
        status: String,
    ) -> Self {
        Self {
            platform,
            event_id: event_id.into(),
            sent_at: Utc::now(),
            payload: PlatformEventPayload::RewardRedemption(RewardRedemption {
                user_id,
                user_name,
                reward_id,
                reward_title,
                reward_cost,
                user_input,
                status,
            }),
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PlatformEventPayload {
    ChatMessage(ChatMessage),
    RewardRedemption(RewardRedemption),
}

impl PlatformEventPayload {
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::ChatMessage(_) => "chat_message",
            Self::RewardRedemption(_) => "reward_redemption",
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

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RewardRedemption {
    pub user_id: String,
    pub user_name: String,
    pub reward_id: String,
    pub reward_title: String,
    pub reward_cost: i64,
    pub user_input: String,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_message_payload_type_name() {
        let event = PlatformEvent::chat_message(
            PlatformId::TWITCH,
            "msg-1",
            "1".to_string(),
            "viewer".to_string(),
            "hello".to_string(),
        );
        assert_eq!(event.payload.type_name(), "chat_message");
        assert_eq!(event.event_id, "msg-1");
        match &event.payload {
            PlatformEventPayload::ChatMessage(msg) => assert_eq!(msg.text, "hello"),
            PlatformEventPayload::RewardRedemption(_) => unreachable!(),
        }
    }

    #[test]
    fn reward_redemption_payload_type_name() {
        let event = PlatformEvent::reward_redemption(
            PlatformId::TWITCH,
            "red-1",
            "1".to_string(),
            "viewer".to_string(),
            "reward-9".to_string(),
            "Spin".to_string(),
            500,
            "please".to_string(),
            "unfulfilled".to_string(),
        );
        assert_eq!(event.payload.type_name(), "reward_redemption");
        assert_eq!(event.event_id, "red-1");
        match &event.payload {
            PlatformEventPayload::ChatMessage(_) => unreachable!(),
            PlatformEventPayload::RewardRedemption(red) => {
                assert_eq!(red.reward_title, "Spin");
                assert_eq!(red.reward_cost, 500);
                assert_eq!(red.status, "unfulfilled");
            }
        }
    }
}
