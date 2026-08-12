#![cfg(test)]

use std::sync::Arc;

use crate::config::runtime::RuntimeConfig;
use crate::config::static_config::StaticConfig;
use crate::config::store::ConfigStore;
use crate::db::inmemory_config::InMemoryConfigRepository;
use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
use crate::db::inmemory_queue::InMemoryQueueRepository;
use crate::random::StandartRandomProvider;
use crate::state::{AppState, AppStateBuilder};

pub async fn test_state() -> AppState {
    test_state_with_queue_repo(Arc::new(InMemoryQueueRepository::new())).await
}

pub async fn test_state_with_queue_repo(queue_repo: Arc<InMemoryQueueRepository>) -> AppState {
    let config_store = Arc::new(ConfigStore::new(
        Arc::new(StaticConfig::test_config()),
        RuntimeConfig::test_runtime("test-key"),
        Arc::new(InMemoryConfigRepository::new()),
    ));
    AppStateBuilder::new(
        StandartRandomProvider,
        config_store,
        Arc::new(InMemoryPlatformCredentialRepository::new()),
    )
    .with_empty_repos()
    .with_queue_repo(queue_repo)
    .build()
    .await
    .expect("failed to build test state")
}
