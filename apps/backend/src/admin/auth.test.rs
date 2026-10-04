use std::time::{Duration, Instant};

use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
use crate::platform::PlatformCredentialService;

use super::*;

fn test_config() -> Option<Arc<TwitchConfig>> {
    let mut config = TwitchConfig::fixture();
    config.client_id = "client_id".to_string();
    config.client_secret = "client_secret".to_string();
    config.broadcaster_id = String::new();
    config.redirect_uri = "https://localhost:8080/callback".to_string();
    config.credentials_redirect_uri = "https://localhost:8080/creds/callback".to_string();
    Some(Arc::new(config))
}

fn test_service(
    config: Option<Arc<TwitchConfig>>,
) -> AdminAuthService<InMemoryPlatformCredentialRepository> {
    AdminAuthService::new(
        config,
        Arc::new(PlatformCredentialService::new(Arc::new(
            InMemoryPlatformCredentialRepository::new(),
        ))),
    )
}

#[test]
fn start_requires_twitch_config() {
    let service = test_service(None);
    assert!(matches!(
        service.start(),
        Err(AdminAuthError::NotConfigured)
    ));
}

#[test]
fn start_returns_redirect_url() {
    let service = test_service(test_config());
    let url = service.start().expect("start should succeed");
    assert!(url.starts_with("https://id.twitch.tv/oauth2/authorize"));
}

fn auth_url_redirect_uri(auth_url: &str) -> String {
    url::Url::parse(auth_url)
        .expect("auth url parses")
        .query_pairs()
        .find(|(key, _)| key == "redirect_uri")
        .map(|(_, value)| value.into_owned())
        .expect("auth url carries redirect_uri")
}

#[test]
fn start_uses_credentials_redirect_uri() {
    let service = test_service(test_config());
    let url = service.start().expect("start should succeed");
    assert_eq!(
        auth_url_redirect_uri(&url),
        "https://localhost:8080/creds/callback"
    );
}

#[test]
fn start_login_uses_login_redirect_uri() {
    let service = test_service(test_config());
    let url = service.start_login().expect("start_login should succeed");
    assert_eq!(
        auth_url_redirect_uri(&url),
        "https://localhost:8080/callback"
    );
}

#[test]
fn start_login_requires_twitch_config() {
    let service = test_service(None);
    assert!(matches!(
        service.start_login(),
        Err(AdminAuthError::NotConfigured)
    ));
}

#[test]
fn concurrent_starts_keep_own_csrf_tickets() {
    let service = test_service(test_config());
    let first = service.start().expect("first start");
    let second = service.start().expect("second start");
    assert_ne!(first, second, "each start must mint its own ticket");

    assert_eq!(service.csrf.len(), 2);
}

#[test]
fn completing_consumes_only_own_ticket() {
    let service = test_service(test_config());
    service.start().expect("first start");
    service.start().expect("second start");

    let first_ticket = service.csrf.first_ticket().expect("has ticket");
    assert!(service.csrf.consume(&first_ticket));
    assert_eq!(service.csrf.len(), 1);
}

#[tokio::test]
async fn unknown_state_is_rejected() {
    let service = test_service(test_config());
    service.start().expect("start");
    assert!(matches!(
        service.complete("code", "not-a-real-state").await,
        Err(AdminAuthError::CsrfMismatch)
    ));
}

#[tokio::test]
async fn expired_ticket_is_pruned_on_complete() {
    let service = test_service(test_config());
    service
        .csrf
        .insert("stale".to_string(), Instant::now() - Duration::from_secs(1));
    assert!(matches!(
        service.complete("code", "stale").await,
        Err(AdminAuthError::CsrfMismatch)
    ));
    assert!(service.csrf.is_empty());
}

#[tokio::test]
async fn ingress_credentials_configuration_lifecycle() {
    let service = test_service(test_config());
    assert!(!service.is_ingress_credentials_configured().await.unwrap());

    service
        .credentials
        .save_credential(PlatformId::TWITCH, "tok")
        .await
        .unwrap();
    assert!(service.is_ingress_credentials_configured().await.unwrap());

    service.revoke_ingress_credentials().await.unwrap();
    assert!(!service.is_ingress_credentials_configured().await.unwrap());
}
