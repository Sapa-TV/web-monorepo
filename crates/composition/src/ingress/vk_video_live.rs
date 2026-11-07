use std::sync::Arc;

use tokio::select;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use vk_video_live::api;
use vk_video_live::error::Error as VkError;
use vk_video_live::events;
use vk_video_live::pubsub::{PUBSUB_URL, PubSub};
use vk_video_live::transport::Transport;

use crate::ingress::platform::{EventSink, PlatformService};
use crate::ingress::vk_video_live_auth::{VkTransport, VkVideoLiveAuthService};
use crate::stream::StreamStatus;
use domain::error::ingress::PlatformError;
use model::consts::ingress::{VK_RECONNECT_INITIAL_DELAY, VK_RECONNECT_MAX_DELAY};
use model::event::PlatformEvent;
use model::platform::{Platform, PlatformCredentialRepository, PlatformId};

#[non_exhaustive]
pub struct VkVideoLivePlatformService<R, T = VkTransport>
where
    R: PlatformCredentialRepository,
    T: Transport,
{
    auth: Arc<VkVideoLiveAuthService<R, T>>,
    stream_status: Arc<StreamStatus>,
}

impl<R, T> VkVideoLivePlatformService<R, T>
where
    R: PlatformCredentialRepository,
    T: Transport,
{
    pub fn new(auth: Arc<VkVideoLiveAuthService<R, T>>, stream_status: Arc<StreamStatus>) -> Self {
        Self {
            auth,
            stream_status,
        }
    }

    fn apply_stream_info(&self, stream: &Option<api::StreamInfo>) {
        let online = stream.as_ref().map_or(Some(false), |info| {
            info.status.as_deref().and_then(events::live_from_status)
        });
        if let Some(online) = online {
            self.stream_status
                .set_platform(PlatformId::VK_VIDEO_LIVE, online);
        }
    }

    async fn consume_loop(
        &self,
        sink: EventSink,
        shutdown: &CancellationToken,
    ) -> Result<(), PlatformError> {
        let channel_url = self.auth.channel_url().await?;
        let bearer = self.auth.bearer().await?;
        let channel = api::channel(self.auth.transport(), &bearer, &channel_url)
            .await
            .map_err(|e| PlatformError::VkApi(e.to_string()))?;
        self.apply_stream_info(&channel.data.stream);
        let ws_channels = channel.data.channel.web_socket_channels;
        let channels = [
            ws_channels.chat,
            ws_channels.private_chat,
            ws_channels.limited_chat,
            ws_channels.channel_points,
            ws_channels.private_channel_points,
            ws_channels.info,
        ]
        .into_iter()
        .flatten()
        .filter(|c| !c.is_empty())
        .collect::<Vec<_>>();
        if channels.is_empty() {
            return Err(PlatformError::VkApi(
                "no ws channels in channel info".to_string(),
            ));
        }
        let ws_token = api::ws_token(self.auth.transport(), &bearer)
            .await
            .map_err(|e| PlatformError::VkApi(e.to_string()))?;

        let mut pubsub = PubSub::connect(PUBSUB_URL, &ws_token)
            .await
            .map_err(map_ws)?;
        for channel in &channels {
            match pubsub.subscribe(channel).await {
                Ok(()) => tracing::info!("vk video live subscribed to {channel}"),
                Err(e) => tracing::warn!("vk video live subscribe to {channel} rejected: {e}"),
            }
        }

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
            tracing::debug!(%frame, "vk pubsub push");
            let parsed =
                events::parse_push(&frame).map_err(|e| PlatformError::Parse(e.to_string()))?;
            let Some(event) = parsed else {
                continue;
            };
            if let events::PushEvent::StreamStatus(status) = &event {
                self.stream_status
                    .set_platform(PlatformId::VK_VIDEO_LIVE, status.online);
                continue;
            }
            let Some(event) = push_event_from(PlatformId::VK_VIDEO_LIVE, event) else {
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

fn push_event_from(platform: PlatformId, event: events::PushEvent) -> Option<PlatformEvent> {
    match event {
        events::PushEvent::ChatMessage(chat) => {
            if chat.is_deleted || chat.is_private {
                return None;
            }
            Some(PlatformEvent::chat_message(
                platform,
                chat.id.to_string(),
                chat.author_id.to_string(),
                chat.author_nick,
                chat.text,
            ))
        }
        events::PushEvent::RewardDemand(demand) => Some(PlatformEvent::reward_redemption(
            platform,
            demand.id.to_string(),
            demand.user_id.to_string(),
            demand.user_nick,
            demand.reward_id,
            String::new(),
            0,
            String::new(),
            demand.status,
        )),
        events::PushEvent::StreamStatus(_) => None,
    }
}

fn map_ws(e: VkError) -> PlatformError {
    match e {
        VkError::Http(m) => PlatformError::WebSocket(m),
        other => PlatformError::WebSocket(other.to_string()),
    }
}

#[cfg(test)]
#[path = "vk_video_live.test.rs"]
mod tests;
