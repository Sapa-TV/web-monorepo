use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use tokio::time::sleep;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use twitch_api::eventsub::channel::ChannelChatMessageV1;
use twitch_api::eventsub::{Event, EventsubWebsocketData, Message, Transport};
use twitch_api::helix::HelixClient;
use twitch_oauth2::{TwitchToken, UserToken};

use crate::config::TwitchConfig;
use crate::error::ingress::PlatformError;
use crate::ingress::event::PlatformEvent;
use crate::ingress::platform::{EventSink, PlatformService};
use crate::ingress::twitch_auth::TwitchAuthService;
use crate::platform::{Platform, PlatformCredentialRepository, PlatformId};

const EVENTSUB_WS_URL: &str = "wss://eventsub.wss.twitch.tv/ws";
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_secs(1);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(60);

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
    pub fn new(config: Arc<TwitchConfig>, credentials_repo: Arc<R>) -> Self {
        Self {
            config: Arc::clone(&config),
            auth: Arc::new(TwitchAuthService::new(config, credentials_repo)),
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
        let (mut ws, _) = connect_async(EVENTSUB_WS_URL)
            .await
            .map_err(|e| PlatformError::WebSocket(e.to_string()))?;

        let session_id = loop {
            let msg = ws
                .next()
                .await
                .ok_or(PlatformError::Disconnected)?
                .map_err(|e| PlatformError::WebSocket(e.to_string()))?;
            let text = msg
                .to_text()
                .map_err(|e| PlatformError::WebSocket(e.to_string()))?;
            let data =
                Event::parse_websocket(text).map_err(|e| PlatformError::Parse(e.to_string()))?;
            if let EventsubWebsocketData::Welcome { payload, .. } = data {
                break payload.session.id.to_string();
            }
        };

        let broadcaster_id = self.config.broadcaster_id.clone();
        let user_id = token
            .user_id()
            .ok_or_else(|| PlatformError::Auth("token has no user_id".to_string()))?
            .to_string();
        let subscription = ChannelChatMessageV1::new(broadcaster_id, user_id);
        let transport = Transport::websocket(session_id);
        helix
            .create_eventsub_subscription(subscription, transport, token)
            .await
            .map_err(|e| PlatformError::Subscription(e.to_string()))?;
        tracing::info!("twitch eventsub subscribed to channel.chat.message");

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
                    if let Event::ChannelChatMessageV1(payload) = payload
                        && let Message::Notification(data) = payload.message
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

        let mut delay = INITIAL_RECONNECT_DELAY;
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
                    delay = (delay * 2).min(MAX_RECONNECT_DELAY);
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
        }
        assert_eq!(event.payload.type_name(), "chat_message");
    }
}
