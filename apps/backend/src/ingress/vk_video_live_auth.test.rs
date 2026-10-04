use std::sync::{Arc, Mutex};

use crate::config::vk_video_live::VkVideoLiveConfig;
use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
use crate::platform::PlatformCredentialService;

use super::*;

#[derive(Debug, Clone)]
struct FakeCall {
    url: String,
    auth: String,
    body: Option<String>,
}

#[derive(Debug)]
struct FakeTransport {
    responses: Mutex<Vec<Result<String, VkError>>>,
    calls: Arc<Mutex<Vec<FakeCall>>>,
}

impl FakeTransport {
    fn new(responses: Vec<Result<String, VkError>>) -> Self {
        Self {
            responses: Mutex::new(responses),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn calls(&self) -> Vec<FakeCall> {
        self.calls.lock().unwrap().clone()
    }
}

impl Transport for FakeTransport {
    async fn post_form(
        &self,
        url: &str,
        basic_auth: &str,
        encoded_body: &str,
    ) -> Result<String, VkError> {
        self.calls.lock().unwrap().push(FakeCall {
            url: url.to_string(),
            auth: basic_auth.to_string(),
            body: Some(encoded_body.to_string()),
        });
        self.responses.lock().unwrap().remove(0)
    }

    async fn get(&self, url: &str, bearer: &str) -> Result<String, VkError> {
        self.calls.lock().unwrap().push(FakeCall {
            url: url.to_string(),
            auth: bearer.to_string(),
            body: None,
        });
        self.responses.lock().unwrap().remove(0)
    }

    async fn post_json(&self, url: &str, bearer: &str, body: &str) -> Result<String, VkError> {
        self.calls.lock().unwrap().push(FakeCall {
            url: url.to_string(),
            auth: bearer.to_string(),
            body: Some(body.to_string()),
        });
        self.responses.lock().unwrap().remove(0)
    }
}

fn config() -> Arc<VkVideoLiveConfig> {
    Arc::new(VkVideoLiveConfig::fixture())
}

fn credentials() -> Arc<PlatformCredentialService<InMemoryPlatformCredentialRepository>> {
    Arc::new(PlatformCredentialService::new(Arc::new(
        InMemoryPlatformCredentialRepository::new(),
    )))
}

const TOKEN_JSON: &str =
    r#"{"access_token":"at1","refresh_token":"rt1","expires_in":3600,"token_type":"Bearer"}"#;
const CHANNEL_JSON: &str = r#"{"data":{"channel":{"id":4242,"url":"test_channel","nick":"TestChannel","web_socket_channels":{"chat":"channel-chat:4242"}},"owner":{"id":555,"nick":"tester"},"stream":null}}"#;

#[tokio::test]
async fn complete_connect_exchanges_saves_and_resolves_channel() {
    let transport = FakeTransport::new(vec![
        Ok(TOKEN_JSON.to_string()),
        Ok(CHANNEL_JSON.to_string()),
    ]);
    let credentials = credentials();
    let service =
        VkVideoLiveAuthService::with_transport(config(), Arc::clone(&credentials), transport);

    let creds = service.complete_connect("the-code").await.unwrap();

    assert_eq!(creds.channel_id, 4242);
    assert_eq!(creds.channel_url, "test_channel");
    assert_eq!(creds.user_id, "555");
    assert_eq!(creds.user_nick, "tester");
    assert_eq!(creds.access_token, "at1");

    let stored = service.load().await.unwrap().expect("creds saved");
    assert_eq!(stored.refresh_token, "rt1");
}

#[tokio::test]
async fn access_token_returns_fresh_token_without_network() {
    let transport = FakeTransport::new(vec![]);
    let credentials = credentials();
    let service =
        VkVideoLiveAuthService::with_transport(config(), Arc::clone(&credentials), transport);

    let mut creds = VkCreds::from_token(
        serde_json::from_str::<TokenResponse>(TOKEN_JSON).unwrap(),
        1,
        "nick",
        4242,
        "test_channel",
    );
    creds.expires_at = Utc::now().timestamp() + 3600;
    service.save(&creds).await.unwrap();

    assert_eq!(service.access_token().await.unwrap(), "at1");
    assert!(service.transport.calls().is_empty());
}

#[tokio::test]
async fn access_token_refreshes_expired_and_rotates_silently() {
    let transport = FakeTransport::new(vec![Ok(
        r#"{"access_token":"at2","refresh_token":"rt2","expires_in":7200,"token_type":"Bearer"}"#
            .to_string(),
    )]);
    let credentials = credentials();
    let service =
        VkVideoLiveAuthService::with_transport(config(), Arc::clone(&credentials), transport);
    let mut creds = VkCreds::from_token(
        serde_json::from_str::<TokenResponse>(TOKEN_JSON).unwrap(),
        1,
        "nick",
        4242,
        "test_channel",
    );
    creds.expires_at = Utc::now().timestamp() - 10;
    service.save(&creds).await.unwrap();
    let mut lifecycle = credentials.subscribe_lifecycle();
    let before = *lifecycle.borrow_and_update();

    assert_eq!(service.access_token().await.unwrap(), "at2");

    let stored = service.load().await.unwrap().unwrap();
    assert_eq!(stored.refresh_token, "rt2");
    assert!(stored.expires_at > Utc::now().timestamp());
    assert_eq!(*lifecycle.borrow(), before);

    let calls = service.transport.calls();
    assert_eq!(calls.len(), 1);
    assert!(
        calls[0].url.ends_with("/oauth/server/token"),
        "{:?}",
        calls[0].url
    );
    assert!(calls[0].auth.starts_with("Basic "));
    assert!(
        calls[0]
            .body
            .as_deref()
            .is_some_and(|body| body.contains("grant_type=refresh_token"))
    );
}

#[tokio::test]
async fn access_token_without_creds_is_auth_error() {
    let transport = FakeTransport::new(vec![]);
    let service = VkVideoLiveAuthService::with_transport(config(), credentials(), transport);
    assert!(matches!(
        service.access_token().await,
        Err(PlatformError::Auth(_))
    ));
}

#[tokio::test]
async fn clear_removes_credentials() {
    let transport = FakeTransport::new(vec![]);
    let credentials = credentials();
    let service =
        VkVideoLiveAuthService::with_transport(config(), Arc::clone(&credentials), transport);

    let creds = VkCreds::from_token(
        serde_json::from_str::<TokenResponse>(TOKEN_JSON).unwrap(),
        1,
        "nick",
        1,
        "u",
    );
    service.save(&creds).await.unwrap();
    assert!(service.load().await.unwrap().is_some());

    service.clear().await.unwrap();
    assert!(service.load().await.unwrap().is_none());
}

#[tokio::test]
async fn platform_auth_trait_reports_status_connect_and_revoke() {
    let transport = FakeTransport::new(vec![
        Ok(TOKEN_JSON.to_string()),
        Ok(CHANNEL_JSON.to_string()),
    ]);
    let credentials = credentials();
    let service = VkVideoLiveAuthService::with_transport(config(), credentials, transport);

    exercise_platform_auth(&service).await;
}

async fn exercise_platform_auth<A: PlatformAuth>(auth: &A) {
    assert_eq!(auth.platform(), PlatformId::VK_VIDEO_LIVE);
    assert!(!auth.is_configured().await.unwrap());

    let identity = auth.connect("the-code").await.unwrap();
    assert_eq!(identity.user_id, "555");
    assert_eq!(identity.user_name, "tester");
    assert!(auth.is_configured().await.unwrap());

    auth.revoke().await.unwrap();
    assert!(!auth.is_configured().await.unwrap());
}
