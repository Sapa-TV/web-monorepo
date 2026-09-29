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
mod tests {
    use std::sync::Arc;

    use vk_video_live::error::Error as VkError;

    use super::*;
    use crate::config::vk_video_live::VkVideoLiveConfig;
    use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
    use crate::platform::PlatformCredentialService;

    const CHANNEL_ONLINE: &str = r#"{"data":{"channel":{"id":4242,"url":"test_channel","nick":"TestChannel","web_socket_channels":{}},"owner":{"id":555,"nick":"tester"},"stream":{"id":"s1","status":"online","started_at":1787501283,"counters":{"viewers":1}}}}"#;
    const CHANNEL_OFFLINE: &str = r#"{"data":{"channel":{"id":4242,"url":"test_channel","nick":"TestChannel","web_socket_channels":{}},"owner":{"id":555,"nick":"tester"},"stream":null}}"#;

    #[derive(Debug)]
    struct FakeTransport {
        responses: Mutex<Vec<Result<String, VkError>>>,
    }

    impl FakeTransport {
        fn new(responses: Vec<Result<String, VkError>>) -> Self {
            Self {
                responses: Mutex::new(responses),
            }
        }
    }

    impl Transport for FakeTransport {
        async fn post_form(
            &self,
            _url: &str,
            _basic_auth: &str,
            _encoded_body: &str,
        ) -> Result<String, VkError> {
            Err(VkError::Protocol("unexpected post_form".to_string()))
        }

        async fn get(&self, _url: &str, _bearer: &str) -> Result<String, VkError> {
            self.responses.lock().remove(0)
        }

        async fn post_json(
            &self,
            _url: &str,
            _bearer: &str,
            _body: &str,
        ) -> Result<String, VkError> {
            self.responses.lock().remove(0)
        }
    }

    type Executor = VkVideoLiveActionExecutor<InMemoryPlatformCredentialRepository, FakeTransport>;

    fn executor(transport: FakeTransport) -> Executor {
        let credentials = Arc::new(PlatformCredentialService::new(Arc::new(
            InMemoryPlatformCredentialRepository::new(),
        )));
        let auth = Arc::new(VkVideoLiveAuthService::with_transport(
            Arc::new(VkVideoLiveConfig::fixture()),
            credentials,
            transport,
        ));
        VkVideoLiveActionExecutor::new("test_channel", auth)
    }

    fn ctx() -> ActionContext {
        ActionContext::new("e1".to_string(), "1".to_string(), "u".to_string())
    }

    fn creds_json() -> String {
        serde_json::json!({
            "access_token": "at",
            "refresh_token": "rt",
            "expires_at": chrono::Utc::now().timestamp() + 3600,
            "user_id": "555",
            "user_nick": "tester",
            "channel_id": 4242,
            "channel_url": "test_channel",
        })
        .to_string()
    }

    async fn executor_with_creds(
        transport: FakeTransport,
    ) -> (
        Executor,
        Arc<PlatformCredentialService<InMemoryPlatformCredentialRepository>>,
    ) {
        let credentials = Arc::new(PlatformCredentialService::new(Arc::new(
            InMemoryPlatformCredentialRepository::new(),
        )));
        credentials
            .save_credential(PlatformId::VK_VIDEO_LIVE, &creds_json())
            .await
            .unwrap();
        let auth = Arc::new(VkVideoLiveAuthService::with_transport(
            Arc::new(VkVideoLiveConfig::fixture()),
            Arc::clone(&credentials),
            transport,
        ));
        (
            VkVideoLiveActionExecutor::new("test_channel", auth),
            credentials,
        )
    }

    #[test]
    fn platform_is_vk_video_live() {
        assert_eq!(
            executor(FakeTransport::new(vec![])).platform(),
            PlatformId::VK_VIDEO_LIVE
        );
    }

    #[tokio::test]
    async fn missing_credentials_maps_to_api_error() {
        let executor = executor(FakeTransport::new(vec![]));
        let err = executor
            .send_chat_message(&ctx(), "hello")
            .await
            .expect_err("no credentials");
        assert!(matches!(err, ActionError::Api(_)));
    }

    #[tokio::test]
    async fn sends_message_with_stream_id() {
        let (executor, _credentials) = executor_with_creds(FakeTransport::new(vec![
            Ok(CHANNEL_ONLINE.to_string()),
            Ok("{}".to_string()),
        ]))
        .await;

        executor
            .send_chat_message(&ctx(), "hi")
            .await
            .expect("send succeeds");
    }

    #[tokio::test]
    async fn second_send_inside_interval_is_rate_limited() {
        let (executor, _credentials) = executor_with_creds(FakeTransport::new(vec![
            Ok(CHANNEL_ONLINE.to_string()),
            Ok("{}".to_string()),
        ]))
        .await;

        executor.send_chat_message(&ctx(), "first").await.unwrap();
        let err = executor
            .send_chat_message(&ctx(), "second")
            .await
            .expect_err("throttled");
        assert!(matches!(err, ActionError::RateLimited));
    }

    #[tokio::test]
    async fn offline_stream_maps_to_api_error() {
        let (executor, _credentials) =
            executor_with_creds(FakeTransport::new(vec![Ok(CHANNEL_OFFLINE.to_string())])).await;

        let err = executor
            .send_chat_message(&ctx(), "hi")
            .await
            .expect_err("offline");
        assert!(matches!(err, ActionError::Api(m) if m.contains("offline")));
    }

    #[tokio::test]
    async fn send_too_fast_response_maps_to_rate_limited() {
        let (executor, _credentials) = executor_with_creds(FakeTransport::new(vec![
            Ok(CHANNEL_ONLINE.to_string()),
            Err(VkError::Http(
                "status 400: {\"error\":\"send_too_fast\"}".to_string(),
            )),
        ]))
        .await;

        let err = executor
            .send_chat_message(&ctx(), "hi")
            .await
            .expect_err("rate limited upstream");
        assert!(matches!(err, ActionError::RateLimited));
    }
}
