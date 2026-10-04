use std::sync::Arc;
use std::sync::nonpoison::Mutex;
use std::time::{Duration, Instant};

use vk_video_live::api;
use vk_video_live::error::Error as VkError;
use vk_video_live::transport::Transport;

use crate::actions::platform::{ActionContext, PlatformActionExecutor};
use crate::error::platform_action::ActionError;
use crate::ingress::vk_video_live_auth::{VkTransport, VkVideoLiveAuthService};
use crate::platform::{PlatformCredentialRepository, PlatformId};

pub const MIN_SEND_INTERVAL: Duration = Duration::from_secs(10);

pub struct VkVideoLiveActionExecutor<R, T = VkTransport>
where
    R: PlatformCredentialRepository,
    T: Transport,
{
    channel_url: String,
    auth: Arc<VkVideoLiveAuthService<R, T>>,
    last_send: Mutex<Option<Instant>>,
}

impl<R, T> VkVideoLiveActionExecutor<R, T>
where
    R: PlatformCredentialRepository,
    T: Transport,
{
    pub fn new(channel_url: impl Into<String>, auth: Arc<VkVideoLiveAuthService<R, T>>) -> Self {
        Self {
            channel_url: channel_url.into(),
            auth,
            last_send: Mutex::new(None),
        }
    }

    fn reserve_send_slot(&self) -> bool {
        let mut last_send = self.last_send.lock();
        let now = Instant::now();
        if let Some(previous) = *last_send
            && now.duration_since(previous) < MIN_SEND_INTERVAL
        {
            return false;
        }
        *last_send = Some(now);
        true
    }
}

impl<R, T> PlatformActionExecutor for VkVideoLiveActionExecutor<R, T>
where
    R: PlatformCredentialRepository,
    T: Transport,
{
    fn platform(&self) -> PlatformId {
        PlatformId::VK_VIDEO_LIVE
    }

    async fn send_chat_message(&self, _ctx: &ActionContext, text: &str) -> Result<(), ActionError> {
        if !self.reserve_send_slot() {
            tracing::warn!(
                platform = self.platform().name(),
                action = "send_chat_message",
                "skipped: vk enforces a min interval between chat messages"
            );
            return Err(ActionError::RateLimited);
        }

        let bearer = self
            .auth
            .bearer()
            .await
            .map_err(|e| ActionError::Api(e.to_string()))?;
        let transport = self.auth.transport();
        let channel = api::channel(transport, &bearer, &self.channel_url)
            .await
            .map_err(map_vk)?;
        let stream_id = channel
            .data
            .stream
            .map(|stream| stream.id)
            .ok_or_else(|| ActionError::Api("vk stream is offline".to_string()))?;

        api::send_chat_message(transport, &bearer, &self.channel_url, &stream_id, text)
            .await
            .map_err(map_vk)
    }
}

fn map_vk(e: VkError) -> ActionError {
    let message = e.to_string();
    if message.contains("send_too_fast") {
        tracing::warn!(
            platform = "vk_video_live",
            "chat send rejected: send_too_fast"
        );
        return ActionError::RateLimited;
    }
    ActionError::Api(message)
}

#[cfg(test)]
#[path = "vk_video_live_executor.test.rs"]
mod tests;
