use std::sync::{Arc, Mutex};

use vk_video_live::error::Error as VkError;
use vk_video_live::transport::Transport;

use crate::config::vk_video_live::VkVideoLiveConfig;
use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
use crate::ingress::vk_video_live_auth::VkVideoLiveAuthService;
use crate::platform::PlatformCredentialService;

use super::*;

type Service = VkVideoLiveAdminAuthService<InMemoryPlatformCredentialRepository, FakeTransport>;

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
        self.responses.lock().unwrap().remove(0)
    }

    async fn get(&self, _url: &str, _bearer: &str) -> Result<String, VkError> {
        self.responses.lock().unwrap().remove(0)
    }

    async fn post_json(&self, _url: &str, _bearer: &str, _body: &str) -> Result<String, VkError> {
        self.responses.lock().unwrap().remove(0)
    }
}

fn config() -> Arc<VkVideoLiveConfig> {
    Arc::new(VkVideoLiveConfig::fixture())
}

fn service(config: Option<Arc<VkVideoLiveConfig>>, transport: FakeTransport) -> Service {
    let credentials = Arc::new(PlatformCredentialService::new(Arc::new(
        InMemoryPlatformCredentialRepository::new(),
    )));
    let auth = config.clone().map(|cfg| {
        Arc::new(VkVideoLiveAuthService::with_transport(
            cfg,
            credentials,
            transport,
        ))
    });
    VkVideoLiveAdminAuthService::new(config, auth)
}

const TOKEN_JSON: &str =
    r#"{"access_token":"at1","refresh_token":"rt1","expires_in":3600,"token_type":"Bearer"}"#;
const CURRENT_USER_JSON: &str =
    r#"{"data":{"user":{"id":555,"nick":"tester"},"channel":{"url":"test_channel"}}}"#;
const CHANNEL_JSON: &str = r#"{"data":{"channel":{"id":4242,"url":"test_channel","nick":"TestChannel","web_socket_channels":{"chat":"channel-chat:4242"}},"owner":{"id":555,"nick":"tester"},"stream":null}}"#;

#[test]
fn start_requires_config() {
    let svc = service(None, FakeTransport::new(vec![]));
    assert!(matches!(svc.start(), Err(AdminAuthError::NotConfigured)));
}

#[test]
fn start_returns_authorize_url_and_stores_state() {
    let svc = service(Some(config()), FakeTransport::new(vec![]));
    let url = svc.start().expect("start");
    assert!(
        url.starts_with("https://auth.live.vkvideo.ru/app/oauth2/authorize"),
        "unexpected url: {url}"
    );
    assert!(url.contains("scope=chat%3Amessage%3Asend"));
    assert!(url.contains("redirect_uri=https%3A%2F%2Flocalhost%2Fcreds-callback%2Fvk-video-live"));
    assert_eq!(svc.csrf.len(), 1);
}

#[tokio::test]
async fn complete_rejects_unknown_state() {
    let svc = service(
        Some(config()),
        FakeTransport::new(vec![
            Ok(TOKEN_JSON.to_string()),
            Ok(CURRENT_USER_JSON.to_string()),
            Ok(CHANNEL_JSON.to_string()),
        ]),
    );
    svc.start().expect("start");
    assert!(matches!(
        svc.complete("code", "unknown-state").await,
        Err(AdminAuthError::CsrfMismatch)
    ));
}

#[tokio::test]
async fn complete_exchanges_connects_and_marks_configured() {
    let svc = service(
        Some(config()),
        FakeTransport::new(vec![
            Ok(TOKEN_JSON.to_string()),
            Ok(CURRENT_USER_JSON.to_string()),
            Ok(CHANNEL_JSON.to_string()),
        ]),
    );
    let url = svc.start().expect("start");
    let state = url
        .split("state=")
        .nth(1)
        .expect("state in url")
        .to_string();

    let identity = svc.complete("the-code", &state).await.unwrap();
    assert_eq!(identity.user_id, "555");
    assert_eq!(identity.user_name, "tester");
    assert!(svc.is_configured().await.unwrap());
    assert_eq!(svc.csrf.len(), 0);
}

#[tokio::test]
async fn revoke_clears_credentials() {
    let svc = service(
        Some(config()),
        FakeTransport::new(vec![
            Ok(TOKEN_JSON.to_string()),
            Ok(CURRENT_USER_JSON.to_string()),
            Ok(CHANNEL_JSON.to_string()),
        ]),
    );
    let url = svc.start().expect("start");
    let state = url.split("state=").nth(1).unwrap().to_string();
    svc.complete("the-code", &state).await.unwrap();

    svc.revoke().await.unwrap();
    assert!(!svc.is_configured().await.unwrap());
}
