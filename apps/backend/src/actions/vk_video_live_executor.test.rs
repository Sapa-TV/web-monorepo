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

    async fn post_json(&self, _url: &str, _bearer: &str, _body: &str) -> Result<String, VkError> {
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
    VkVideoLiveActionExecutor::new(auth)
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
    (VkVideoLiveActionExecutor::new(auth), credentials)
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
