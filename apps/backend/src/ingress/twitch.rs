use std::sync::Arc;

use futures_util::StreamExt;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use twitch_api::eventsub::channel::channel_points_custom_reward_redemption::RedemptionStatus;
use twitch_api::eventsub::channel::{
    ChannelChatMessageV1, ChannelPointsCustomRewardRedemptionAddV1,
    ChannelPointsCustomRewardRedemptionAddV1Payload,
};
use twitch_api::eventsub::{Event, EventsubWebsocketData, Message, Transport};
use twitch_api::helix::HelixClient;
use twitch_oauth2::{TwitchToken, UserToken};

use crate::config::TwitchConfig;
use crate::consts::ingress::{
    TWITCH_EVENTSUB_WS_URL, TWITCH_RECONNECT_INITIAL_DELAY, TWITCH_RECONNECT_MAX_DELAY,
};
use crate::error::ingress::PlatformError;
use crate::ingress::event::PlatformEvent;
use crate::ingress::platform::{EventSink, PlatformService};
use crate::ingress::twitch_auth::TwitchAuthService;
use crate::platform::{
    Platform, PlatformCredentialRepository, PlatformCredentialService, PlatformId,
};

#[non_exhaustive]
pub struct TwitchPlatformService<R>
where
    R: PlatformCredentialRepository,
{
    config: Arc<TwitchConfig>,
    auth: Arc<TwitchAuthService<R>>,
    platform: PlatformId,
}

impl<R> TwitchPlatformService<R>
where
    R: PlatformCredentialRepository,
{
    pub fn new(config: Arc<TwitchConfig>, credentials: Arc<PlatformCredentialService<R>>) -> Self {
        Self {
            config: Arc::clone(&config),
            auth: Arc::new(TwitchAuthService::new(config, credentials)),
            platform: PlatformId::TWITCH,
        }
    }

    async fn consume_loop(
        &self,
        helix: &HelixClient<'static, reqwest::Client>,
        token: &UserToken,
        sink: EventSink,
        platform: PlatformId,
    ) -> Result<(), PlatformError> {
        let (mut ws, _) = connect_async(TWITCH_EVENTSUB_WS_URL)
            .await
            .map_err(|e| PlatformError::WebSocket(e.to_string()))?;

        let session_id = loop {
            let msg = ws
                .next()
                .await
                .ok_or(PlatformError::Disconnected)?
                .map_err(|e| PlatformError::WebSocket(e.to_string()))?;
            let text = match msg {
                WsMessage::Text(text) => text,
                WsMessage::Close(_) => return Err(PlatformError::Disconnected),
                _ => continue,
            };
            let data =
                Event::parse_websocket(&text).map_err(|e| PlatformError::Parse(e.to_string()))?;
            if let EventsubWebsocketData::Welcome { payload, .. } = data {
                break payload.session.id.to_string();
            }
        };

        let broadcaster_id = self.config.broadcaster_id.clone();
        let user_id = token
            .user_id()
            .ok_or_else(|| PlatformError::Auth("token has no user_id".to_string()))?
            .to_string();
        let transport = Transport::websocket(session_id);
        let subscription = ChannelChatMessageV1::new(broadcaster_id.clone(), user_id);
        helix
            .create_eventsub_subscription(subscription, transport.clone(), token)
            .await
            .map_err(|e| PlatformError::Subscription(e.to_string()))?;
        tracing::info!("twitch eventsub subscribed to channel.chat.message");

        let redemption_subscription =
            ChannelPointsCustomRewardRedemptionAddV1::broadcaster_user_id(broadcaster_id);
        helix
            .create_eventsub_subscription(redemption_subscription, transport, token)
            .await
            .map_err(|e| PlatformError::Subscription(e.to_string()))?;
        tracing::info!(
            "twitch eventsub subscribed to channel.channel_points_custom_reward_redemption.add"
        );

        while let Some(msg) = ws.next().await {
            let msg = msg.map_err(|e| PlatformError::WebSocket(e.to_string()))?;
            let text = match msg {
                WsMessage::Text(text) => text,
                WsMessage::Close(_) => return Err(PlatformError::Disconnected),
                _ => continue,
            };
            let data = match Event::parse_websocket(&text) {
                Ok(data) => data,
                Err(e) => {
                    tracing::warn!("failed to parse twitch eventsub frame: {e}");
                    continue;
                }
            };
            match data {
                EventsubWebsocketData::Notification { payload, .. } => {
                    if let Event::ChannelChatMessageV1(payload) = &payload
                        && let Message::Notification(data) = &payload.message
                    {
                        let event = chat_event_from(
                            platform,
                            data.message_id.as_ref(),
                            data.chatter_user_id.as_ref(),
                            data.chatter_user_name.as_ref(),
                            data.message.text.as_str(),
                        );
                        if sink.send(event).await.is_err() {
                            return Err(PlatformError::SinkClosed);
                        }
                    }
                    if let Event::ChannelPointsCustomRewardRedemptionAddV1(payload) = &payload
                        && let Message::Notification(data) = &payload.message
                    {
                        let event = reward_redemption_event_from(platform, data);
                        if sink.send(event).await.is_err() {
                            return Err(PlatformError::SinkClosed);
                        }
                    }
                }
                EventsubWebsocketData::Keepalive { .. } => {}
                EventsubWebsocketData::Reconnect { .. } => {
                    tracing::info!("twitch eventsub requested reconnect");
                    return Err(PlatformError::Disconnected);
                }
                EventsubWebsocketData::Revocation { .. } => {
                    tracing::warn!("twitch eventsub subscription revoked");
                    return Err(PlatformError::Disconnected);
                }
                EventsubWebsocketData::Welcome { .. } => {}
                _ => {}
            }
        }
        Err(PlatformError::Disconnected)
    }
}

impl<R> PlatformService for TwitchPlatformService<R>
where
    R: PlatformCredentialRepository,
{
    fn platform(&self) -> Platform {
        Platform::from_id(self.platform)
    }

    async fn run(&self, sink: EventSink) -> Result<(), PlatformError> {
        let helix = self.auth.helix();

        let mut delay = TWITCH_RECONNECT_INITIAL_DELAY;
        loop {
            let token = self.auth.user_token().await?;
            match self
                .consume_loop(&helix, &token, sink.clone(), self.platform)
                .await
            {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!("twitch eventsub stopped: {e}; reconnecting in {delay:?}");
                    sleep(delay).await;
                    delay = (delay * 2).min(TWITCH_RECONNECT_MAX_DELAY);
                }
            }
        }
    }
}

fn chat_event_from(
    platform: PlatformId,
    event_id: &str,
    user_id: &str,
    user_name: &str,
    text: &str,
) -> PlatformEvent {
    PlatformEvent::chat_message(
        platform,
        event_id.to_owned(),
        user_id.to_owned(),
        user_name.to_owned(),
        text.to_owned(),
    )
}

fn reward_redemption_event_from(
    platform: PlatformId,
    data: &ChannelPointsCustomRewardRedemptionAddV1Payload,
) -> PlatformEvent {
    PlatformEvent::reward_redemption(
        platform,
        data.id.as_str().to_string(),
        data.user_id.as_str().to_string(),
        data.user_name.as_str().to_string(),
        data.reward.id.as_str().to_string(),
        data.reward.title.clone(),
        data.reward.cost,
        data.user_input.clone(),
        redemption_status_to_str(&data.status).to_string(),
    )
}

fn redemption_status_to_str(status: &RedemptionStatus) -> &'static str {
    match status {
        RedemptionStatus::Unfulfilled => "unfulfilled",
        RedemptionStatus::Fulfilled => "fulfilled",
        RedemptionStatus::Canceled => "canceled",
        RedemptionStatus::Unknown | _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ingress::event::PlatformEventPayload;

    #[test]
    fn maps_chat_message_fields() {
        let event = chat_event_from(
            PlatformId::TWITCH,
            "msg-1",
            "4145994",
            "viewer32",
            "Hi chat",
        );
        assert_eq!(event.platform, PlatformId::TWITCH);
        assert_eq!(event.event_id, "msg-1");
        match &event.payload {
            PlatformEventPayload::ChatMessage(msg) => {
                assert_eq!(msg.user_id, "4145994");
                assert_eq!(msg.user_name, "viewer32");
                assert_eq!(msg.text, "Hi chat");
            }
            PlatformEventPayload::RewardRedemption(_) => unreachable!(),
        }
        assert_eq!(event.payload.type_name(), "chat_message");
    }

    #[test]
    fn maps_reward_redemption_fields() {
        let data: ChannelPointsCustomRewardRedemptionAddV1Payload = serde_json::from_str(
            r##"{
                    "id": "1234",
                    "broadcaster_user_id": "1337",
                    "broadcaster_user_login": "cool_user",
                    "broadcaster_user_name": "Cool_User",
                    "user_id": "9001",
                    "user_login": "cooler_user",
                    "user_name": "Cooler_User",
                    "user_input": "pogchamp",
                    "status": "unfulfilled",
                    "reward": {
                        "id": "9001",
                        "title": "title",
                        "cost": 100,
                        "prompt": "reward prompt"
                    },
                    "redeemed_at": "2020-07-15T17:16:03.17106713Z"
                }"##,
        )
        .expect("payload should deserialize");
        let event = reward_redemption_event_from(PlatformId::TWITCH, &data);
        assert_eq!(event.event_id, "1234");
        match &event.payload {
            PlatformEventPayload::ChatMessage(_) => unreachable!(),
            PlatformEventPayload::RewardRedemption(red) => {
                assert_eq!(red.user_id, "9001");
                assert_eq!(red.user_name, "Cooler_User");
                assert_eq!(red.reward_id, "9001");
                assert_eq!(red.reward_title, "title");
                assert_eq!(red.reward_cost, 100);
                assert_eq!(red.user_input, "pogchamp");
                assert_eq!(red.status, "unfulfilled");
            }
        }
        assert_eq!(event.payload.type_name(), "reward_redemption");
    }
}
