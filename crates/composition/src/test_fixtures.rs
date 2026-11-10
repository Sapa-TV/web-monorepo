use std::sync::Arc;

use sqlx::SqlitePool;

use crate::config::static_config::StaticConfig;
use crate::platform::PlatformId;
use crate::state::{AppConfigStore, AppState, AppStateBuilder};
use domain::config::settings::SharedSettings;
use domain::queue::publisher::BroadcastEventPublisher;
use domain::queue::service::QueueService;
use domain::random::StandartRandomProvider;
use domain::roulette::machine::RouletteService;
use domain::roulette::rarity_service::RarityService;
use domain::roulette::slot_service::RouletteSlotService;
use domain::user::service::UserService;
use model::config::runtime::RuntimeConfig;
use storage_inmemory::inmemory_platform::InMemoryPlatformRepository;
use storage_inmemory::inmemory_queue::InMemoryQueueRepository;
use storage_inmemory::inmemory_rarity::InMemoryRarityRepository;
use storage_inmemory::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
use storage_inmemory::inmemory_user::InMemoryUserRepository;
use storage_sqlite::config::SqliteConfigRepository;
use storage_sqlite::platform_credential::SqlitePlatformCredentialRepository;
use storage_sqlite::queue::SqliteQueueRepository;
use storage_sqlite::test_pool;

/// Empty slots/rarities (like pre-sqlite `with_empty_repos`), default queue repo.
pub async fn test_state() -> AppState {
    let (pool, _path) = test_pool().await;
    build_state(pool, false, None, StaticConfig::test_config()).await
}

/// Same as [`test_state`] but with a custom static configuration.
pub async fn test_state_with_static(static_cfg: StaticConfig) -> AppState {
    let (pool, _path) = test_pool().await;
    build_state(pool, false, None, static_cfg).await
}

/// Empty slots/rarities with an explicit pool and optional queue repo override.
pub async fn test_state_with(
    pool: SqlitePool,
    queue_repo: Option<Arc<SqliteQueueRepository>>,
) -> AppState {
    build_state(pool, false, queue_repo, StaticConfig::test_config()).await
}

#[non_exhaustive]
pub struct InMemoryServices {
    pub slot_service: Arc<RouletteSlotService<Arc<InMemoryRouletteSlotRepository>>>,
    pub rarity_service: Arc<RarityService<Arc<InMemoryRarityRepository>>>,
    pub user_service: Arc<UserService<InMemoryUserRepository, InMemoryPlatformRepository>>,
    pub queue_service: Arc<
        QueueService<
            InMemoryQueueRepository,
            InMemoryRarityRepository,
            InMemoryRouletteSlotRepository,
        >,
    >,
}

/// No sqlite at all: just the services the queue and action tests drive.
pub async fn test_services_inmemory() -> (InMemoryServices, Arc<InMemoryQueueRepository>) {
    let queue_repo = Arc::new(InMemoryQueueRepository::new());
    let settings = SharedSettings::new(RuntimeConfig::test_runtime("test-key"));

    let slot_service = Arc::new(
        RouletteSlotService::build(Arc::new(InMemoryRouletteSlotRepository::new()))
            .await
            .expect("failed to build in-memory slot service"),
    );
    let rarity_service = Arc::new(
        RarityService::build(Arc::new(InMemoryRarityRepository::new()))
            .await
            .expect("failed to build in-memory rarity service"),
    );
    let roulette = RouletteService::new(Arc::clone(&slot_service), StandartRandomProvider::new());
    let queue_service = Arc::new(QueueService::new(
        Arc::clone(&queue_repo),
        Arc::clone(&rarity_service),
        roulette,
        BroadcastEventPublisher::new(),
        settings,
    ));
    let user_service = Arc::new(UserService::new(
        Arc::new(InMemoryUserRepository::new()),
        Arc::new(InMemoryPlatformRepository::new_seeded()),
    ));

    (
        InMemoryServices {
            slot_service,
            rarity_service,
            user_service,
            queue_service,
        },
        queue_repo,
    )
}

async fn build_state(
    pool: SqlitePool,
    seeded: bool,
    queue_repo: Option<Arc<SqliteQueueRepository>>,
    static_cfg: StaticConfig,
) -> AppState {
    let config_store = Arc::new(AppConfigStore::new(
        Arc::new(static_cfg),
        RuntimeConfig::test_runtime("test-key"),
        Arc::new(SqliteConfigRepository::new(pool.clone())),
    ));
    let credentials_repo = Arc::new(SqlitePlatformCredentialRepository::new(pool.clone()));

    let mut builder = AppStateBuilder::new(
        StandartRandomProvider::new(),
        config_store,
        credentials_repo,
        pool,
    );
    if !seeded {
        builder = builder.with_empty_repos();
    }
    if let Some(queue_repo) = queue_repo {
        builder = builder.with_queue_repo(queue_repo);
    }
    builder.build().await.expect("failed to build test state")
}

pub async fn save_twitch_credentials(state: &AppState, token: &str) {
    state
        .credentials
        .save_credential(PlatformId::TWITCH, token)
        .await
        .expect("failed to save twitch credentials");
}
