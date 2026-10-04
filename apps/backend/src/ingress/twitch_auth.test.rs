use std::sync::Arc;

use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
use crate::platform::{PlatformCredentialService, PlatformId};

use super::*;

fn test_config() -> Arc<TwitchConfig> {
    let mut config = TwitchConfig::fixture();
    config.client_id = "id".to_string();
    config.client_secret = "secret".to_string();
    config.broadcaster_id = "broadcaster".to_string();
    config.redirect_uri = String::new();
    config.credentials_redirect_uri = String::new();
    Arc::new(config)
}

fn test_credentials() -> Arc<PlatformCredentialService<InMemoryPlatformCredentialRepository>> {
    Arc::new(PlatformCredentialService::new(Arc::new(
        InMemoryPlatformCredentialRepository::new(),
    )))
}

#[tokio::test]
async fn current_refresh_token_reads_from_repo() {
    let credentials = test_credentials();
    credentials
        .save_rotated(PlatformId::TWITCH, "from_repo")
        .await
        .unwrap();
    let service = TwitchAuthService::new(test_config(), credentials);
    assert_eq!(service.current_refresh_token().await.unwrap(), "from_repo");
}

#[tokio::test]
async fn current_refresh_token_errors_when_repo_empty() {
    let service = TwitchAuthService::new(test_config(), test_credentials());
    assert!(matches!(
        service.current_refresh_token().await,
        Err(PlatformError::Auth(_))
    ));
}

#[tokio::test]
async fn current_refresh_token_reads_replaced_credential() {
    let credentials = test_credentials();
    credentials
        .save_credential(PlatformId::TWITCH, "first")
        .await
        .unwrap();
    let service = TwitchAuthService::new(test_config(), credentials);

    assert_eq!(service.current_refresh_token().await.unwrap(), "first");

    service
        .credentials
        .save_credential(PlatformId::TWITCH, "second")
        .await
        .unwrap();
    assert_eq!(service.current_refresh_token().await.unwrap(), "second");
}

#[tokio::test]
async fn save_rotated_does_not_bump_lifecycle() {
    let credentials = test_credentials();
    let rx = credentials.subscribe_lifecycle();
    credentials
        .save_rotated(PlatformId::TWITCH, "rotated")
        .await
        .unwrap();
    assert_eq!(*rx.borrow(), 0);
}
