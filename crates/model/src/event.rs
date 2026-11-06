use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;
use utoipa::ToSchema;

use crate::platform::PlatformId;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PlatformEvent {
    pub platform: PlatformId,
    pub event_id: String,
    pub sent_at: DateTime<Utc>,
    pub payload: PlatformEventPayload,
    _sealed: (),
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
                _sealed: (),
            }),
            _sealed: (),
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
                _sealed: (),
            }),
            _sealed: (),
        }
    }
}

#[derive(Debug, Clone, EnumDiscriminants)]
#[non_exhaustive]
#[strum_discriminants(
    derive(
        Serialize,
        Deserialize,
        ToSchema,
        strum::EnumString,
        strum::IntoStaticStr
    ),
    serde(rename_all = "snake_case"),
    strum(serialize_all = "snake_case")
)]
#[strum_discriminants(name(RuleTrigger))]
pub enum PlatformEventPayload {
    ChatMessage(ChatMessage),
    RewardRedemption(RewardRedemption),
}

impl PlatformEventPayload {
    pub fn type_name(&self) -> &'static str {
        use strum::IntoDiscriminant;
        let trigger = self.discriminant();
        <&'static str>::from(&trigger)
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ChatMessage {
    pub user_id: String,
    pub user_name: String,
    pub text: String,
    _sealed: (),
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
    _sealed: (),
}

#[cfg(test)]
#[path = "event.test.rs"]
mod tests;
