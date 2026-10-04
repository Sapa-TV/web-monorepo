use std::sync::Arc;

use super::*;
use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
use crate::platform::PlatformCredentialService;

fn executor_without_credentials() -> TwitchActionExecutor<InMemoryPlatformCredentialRepository> {
    let config = Arc::new(TwitchConfig::fixture());

    let credentials = Arc::new(PlatformCredentialService::new(Arc::new(
        InMemoryPlatformCredentialRepository::new(),
    )));
    let auth = Arc::new(TwitchAuthService::new(Arc::clone(&config), credentials));
    TwitchActionExecutor::new(config, auth)
}

#[tokio::test]
async fn platform_is_twitch() {
    assert_eq!(
        executor_without_credentials().platform(),
        PlatformId::TWITCH
    );
}

#[tokio::test]
async fn missing_token_maps_to_api_error() {
    let executor = executor_without_credentials();
    let ctx = ActionContext::new("e".to_string(), "1".to_string(), "u".to_string());

    let err = executor
        .send_chat_message(&ctx, "hello")
        .await
        .expect_err("no credentials stored");

    assert!(matches!(err, ActionError::Api(_)));
}
