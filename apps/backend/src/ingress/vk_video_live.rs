use std::sync::Arc;

use tokio::select;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use vk_video_live::api;
use vk_video_live::error::Error as VkError;
use vk_video_live::events::{self, ChatMessageEvent};
use vk_video_live::pubsub::{PUBSUB_URL, PubSub};
use vk_video_live::transport::Transport;

use crate::consts::ingress::{VK_RECONNECT_INITIAL_DELAY, VK_RECONNECT_MAX_DELAY};
use crate::error::ingress::PlatformError;
use crate::ingress::event::PlatformEvent;
use crate::ingress::platform::{EventSink, PlatformService};
use crate::ingress::vk_video_live_auth::{VkTransport, VkVideoLiveAuthService};
use crate::platform::{Platform, PlatformCredentialRepository, PlatformId};

#[non_exhaustive]
pub struct VkVideoLivePlatformService<R, T = VkTransport>
where
    R: PlatformCredentialRepository,
    T: Transport,
{
    auth: Arc<VkVideoLiveAuthService<R, T>>,
}

impl<R, T> VkVideoLivePlatformService<R, T>
where
    R: PlatformCredentialRepository,
    T: Transport,
{
    pub fn new(auth: Arc<VkVideoLiveAuthService<R, T>>) -> Self {
        Self { auth }
    }

    async fn consume_loop(
        &self,
        sink: EventSink,
        shutdown: &CancellationToken,
    ) -> Result<(), PlatformError> {
        let channel_id = self.auth.channel_id().await?;
        let bearer = self.auth.bearer().await?;
        let ws_token = api::ws_token(self.auth.transport(), &bearer)
            .await
            .map_err(|e| PlatformError::VkApi(e.to_string()))?;

        let mut pubsub = PubSub::connect(PUBSUB_URL, &ws_token)
            .await
            .map_err(map_ws)?;
        pubsub
            .subscribe(&format!("channel-chat:{channel_id}"))
            .await
            .map_err(map_ws)?;
        tracing::info!("vk video live subscribed to channel-chat:{channel_id}");

        loop {
            let frame = select! {
                biased;
                _ = shutdown.cancelled() => {
                    pubsub.close().await;
                    return Err(PlatformError::Cancelled);
                }
                frame = pubsub.next_push() => frame,
            };
            let frame = frame.map_err(map_ws)?;
            let parsed =
                events::parse_push(&frame).map_err(|e| PlatformError::Parse(e.to_string()))?;
            let Some(event) = parsed.and_then(|e| chat_event_from(PlatformId::VK_VIDEO_LIVE, &e))
            else {
                continue;
            };
            if sink.send(event).await.is_err() {
                return Err(PlatformError::SinkClosed);
            }
        }
    }
}

impl<R, T> PlatformService for VkVideoLivePlatformService<R, T>
where
    R: PlatformCredentialRepository,
    T: Transport,
{
    fn platform(&self) -> Platform {
        Platform::from_id(PlatformId::VK_VIDEO_LIVE)
    }

    async fn run(&self, sink: EventSink, shutdown: CancellationToken) -> Result<(), PlatformError> {
        let mut delay = VK_RECONNECT_INITIAL_DELAY;
        loop {
            if shutdown.is_cancelled() {
                return Ok(());
            }
            select! {
                biased;
                _ = shutdown.cancelled() => return Ok(()),
                outcome = self.consume_loop(sink.clone(), &shutdown) => {
                    match outcome {
                        Err(PlatformError::Cancelled) => return Ok(()),
                        Ok(()) => return Ok(()),
                        Err(e) => {
                            tracing::warn!(
                                "vk video live stopped: {e}; reconnecting in {delay:?}"
                            );
                            select! {
                                biased;
                                _ = shutdown.cancelled() => return Ok(()),
                                _ = sleep(delay) => {},
                            }
                            delay = (delay * 2).min(VK_RECONNECT_MAX_DELAY);
                        }
                    }
                }
            }
        }
    }
}

fn chat_event_from(platform: PlatformId, event: &ChatMessageEvent) -> Option<PlatformEvent> {
    if event.is_deleted || event.is_private {
        return None;
    }
    Some(PlatformEvent::chat_message(
        platform,
        event.id.to_string(),
        event.author_id.to_string(),
        event.author_nick.clone(),
        event.text.clone(),
    ))
}

fn map_ws(e: VkError) -> PlatformError {
    match e {
        VkError::Http(m) => PlatformError::WebSocket(m),
        other => PlatformError::WebSocket(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingress::event::PlatformEventPayload;

    fn event(id: u64, nick: &str, text: &str) -> ChatMessageEvent {
        ChatMessageEvent::new(id, 555, nick, 1_787_682_950, text, false, false)
    }

    #[test]
    fn normal_message_maps_to_platform_event() {
        let mapped = chat_event_from(PlatformId::VK_VIDEO_LIVE, &event(651, "tester", "hi"))
            .expect("mapped");
        assert_eq!(mapped.platform, PlatformId::VK_VIDEO_LIVE);
        assert_eq!(mapped.event_id, "651");
        match &mapped.payload {
            PlatformEventPayload::ChatMessage(msg) => {
                assert_eq!(msg.user_id, "555");
                assert_eq!(msg.user_name, "tester");
                assert_eq!(msg.text, "hi");
            }
            other => panic!("expected chat message payload, got {other:?}"),
        }
    }

    #[test]
    fn deleted_message_is_skipped() {
        let mut e = event(1, "n", "t");
        e.is_deleted = true;
        assert!(chat_event_from(PlatformId::VK_VIDEO_LIVE, &e).is_none());
    }

    #[test]
    fn private_message_is_skipped() {
        let mut e = event(1, "n", "t");
        e.is_private = true;
        assert!(chat_event_from(PlatformId::VK_VIDEO_LIVE, &e).is_none());
    }
}
