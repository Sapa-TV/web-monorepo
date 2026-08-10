#![cfg(test)]

use std::sync::Arc;

use crate::config::Config;
use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
use crate::random::StandartRandomProvider;
use crate::state::{AppState, AppStateBuilder};

pub async fn test_state() -> AppState {
    let config = Arc::new(Config::test_config());
    AppStateBuilder::new(
        StandartRandomProvider,
        &config,
        Arc::new(InMemoryPlatformCredentialRepository::new()),
    )
    .with_empty_repos()
    .build()
    .await
    .expect("failed to build test state")
}
