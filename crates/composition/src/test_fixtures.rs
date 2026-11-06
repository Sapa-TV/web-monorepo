use std::sync::Arc;

use sqlx::SqlitePool;

use crate::admin::auth::AdminAuthService;
use crate::admin::vk_auth::VkVideoLiveAdminAuthService;
use crate::config::static_config::StaticConfig;
use crate::config::store::ConfigStore;
use crate::ingress::EventIngress;
use crate::ingress::twitch_auth::TwitchAuthService;
use crate::ingress::vk_video_live_auth::VkVideoLiveAuthService;
use crate::platform::PlatformCredentialService;
use crate::platform::PlatformId;
use crate::presence::Presence;
use crate::sheets::client::GoogleSheetsClient;
use crate::state::{
    AppConfigStore, AppState, AppStateBuilder, UniAppState, UniStateParams, assemble_uni_state,
};
use crate::stream::StreamStatus;
use domain::random::StandartRandomProvider;
use model::config::runtime::RuntimeConfig;
use storage_inmemory::inmemory_actions::InMemoryActionRepository;
use storage_inmemory::inmemory_admin::InMemoryAdminRepository;
use storage_inmemory::inmemory_config::InMemoryConfigRepository;
use storage_inmemory::inmemory_orders::{
    InMemoryGameOrderRepository, InMemoryMovieOrderRepository, InMemoryVipRecordRepository,
};
use storage_inmemory::inmemory_platform::InMemoryPlatformRepository;
use storage_inmemory::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
use storage_inmemory::inmemory_queue::InMemoryQueueRepository;
use storage_inmemory::inmemory_rarity::InMemoryRarityRepository;
use storage_inmemory::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
use storage_inmemory::inmemory_rules::InMemoryRuleRepository;
use storage_inmemory::inmemory_session::InMemorySessionRepository;
use storage_inmemory::inmemory_user::InMemoryUserRepository;
use storage_sqlite::config::SqliteConfigRepository;
use storage_sqlite::platform_credential::SqlitePlatformCredentialRepository;
use storage_sqlite::queue::SqliteQueueRepository;
use storage_sqlite::test_pool;

/// Empty slots/rarities (like pre-sqlite `with_empty_repos`), default queue repo.
pub async fn test_state() -> AppState {
    let (pool, _path) = test_pool().await;
    build_state(pool, false, None).await
}

/// Empty slots/rarities with an explicit pool and optional queue repo override.
pub async fn test_state_with(
    pool: SqlitePool,
    queue_repo: Option<Arc<SqliteQueueRepository>>,
) -> AppState {
    build_state(pool, false, queue_repo).await
}

pub type InMemoryAppState = UniAppState<
    InMemoryQueueRepository,
    InMemoryRarityRepository,
    InMemoryUserRepository,
    InMemoryPlatformRepository,
    InMemoryRouletteSlotRepository,
    InMemoryAdminRepository,
    InMemorySessionRepository,
    InMemoryRuleRepository,
    InMemoryActionRepository,
    InMemoryGameOrderRepository,
    InMemoryMovieOrderRepository,
    InMemoryVipRecordRepository,
    ConfigStore<InMemoryConfigRepository>,
    GoogleSheetsClient,
    PlatformCredentialService<InMemoryPlatformCredentialRepository>,
    AdminAuthService<InMemoryPlatformCredentialRepository>,
    VkVideoLiveAdminAuthService<InMemoryPlatformCredentialRepository>,
    TwitchAuthService<InMemoryPlatformCredentialRepository>,
    VkVideoLiveAuthService<InMemoryPlatformCredentialRepository>,
    EventIngress,
    Presence,
    StreamStatus,
>;

/// No sqlite at all. Platforms are seeded (like migrations do); slots/rarities
/// start empty. Returns the queue repo for tests that need a direct handle.
pub async fn test_state_inmemory() -> (InMemoryAppState, Arc<InMemoryQueueRepository>) {
    let queue_repo = Arc::new(InMemoryQueueRepository::new());
    let config_store = Arc::new(ConfigStore::new(
        Arc::new(StaticConfig::test_config()),
        RuntimeConfig::test_runtime("test-key"),
        Arc::new(InMemoryConfigRepository::new()),
    ));
    let credentials_repo = Arc::new(InMemoryPlatformCredentialRepository::new());
    let credentials = Arc::new(PlatformCredentialService::new(Arc::clone(
        &credentials_repo,
    )));
    let admin_auth = Arc::new(AdminAuthService::new(
        config_store.twitch().map(|twitch| Arc::new(twitch.clone())),
        Arc::clone(&credentials),
    ));
    let vk_api = config_store.vk_video_live().map(|vk| {
        Arc::new(VkVideoLiveAuthService::new(
            Arc::new(vk.clone()),
            Arc::clone(&credentials),
        ))
    });
    let vk_admin_auth = Arc::new(VkVideoLiveAdminAuthService::new(None, vk_api.clone()));
    let twitch_api = config_store.twitch().map(|twitch| {
        Arc::new(TwitchAuthService::new(
            Arc::new(twitch.clone()),
            Arc::clone(&credentials),
        ))
    });
    let settings = config_store.source();
    let state = assemble_uni_state(UniStateParams {
        random: StandartRandomProvider::new(),
        settings,
        config: config_store,
        sheets_client: None,
        credential_service: credentials,
        admin_auth,
        vk_admin_auth,
        twitch_api,
        vk_api,
        ingress: Arc::new(EventIngress::new()),
        presence: Presence::new(),
        stream_status: Arc::new(StreamStatus::new()),
        slot_repo: Arc::new(InMemoryRouletteSlotRepository::new()),
        rarity_repo: Arc::new(InMemoryRarityRepository::new()),
        user_repo: Arc::new(InMemoryUserRepository::new()),
        platform_repo: Arc::new(InMemoryPlatformRepository::new_seeded()),
        queue_repo: Arc::clone(&queue_repo),
        admin_repo: Arc::new(InMemoryAdminRepository::new()),
        session_repo: Arc::new(InMemorySessionRepository::new()),
        rule_repo: Arc::new(InMemoryRuleRepository::new()),
        action_repo: Arc::new(InMemoryActionRepository::new()),
        game_order_repo: Arc::new(InMemoryGameOrderRepository::new()),
        movie_order_repo: Arc::new(InMemoryMovieOrderRepository::new()),
        vip_record_repo: Arc::new(InMemoryVipRecordRepository::new()),
        admin_twitch_id: None,
        extra_admin_twitch_ids: Vec::new(),
    })
    .await
    .expect("failed to build in-memory test state");
    (state, queue_repo)
}

async fn build_state(
    pool: SqlitePool,
    seeded: bool,
    queue_repo: Option<Arc<SqliteQueueRepository>>,
) -> AppState {
    let config_store = Arc::new(AppConfigStore::new(
        Arc::new(StaticConfig::test_config()),
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
        .expect("failed to save twitch credentials")
}
