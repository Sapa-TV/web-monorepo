#![cfg(test)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::header;
use axum::http::{Request, StatusCode};
use sqlx::SqlitePool;
use tower::ServiceExt;

use crate::api;
use crate::api::auth::LOGIN_COOKIE;
use crate::config::runtime::RuntimeConfig;
use crate::config::static_config::StaticConfig;
use crate::config::store::ConfigStore;
use crate::db::inmemory_actions::InMemoryActionRepository;
use crate::db::inmemory_admin::InMemoryAdminRepository;
use crate::db::inmemory_config::InMemoryConfigRepository;
use crate::db::inmemory_orders::{
    InMemoryGameOrderRepository, InMemoryMovieOrderRepository, InMemoryVipRecordRepository,
};
use crate::db::inmemory_platform::InMemoryPlatformRepository;
use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
use crate::db::inmemory_queue::InMemoryQueueRepository;
use crate::db::inmemory_rarity::InMemoryRarityRepository;
use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
use crate::db::inmemory_rules::InMemoryRuleRepository;
use crate::db::inmemory_session::InMemorySessionRepository;
use crate::db::inmemory_user::InMemoryUserRepository;
use crate::db::sqlite::config::SqliteConfigRepository;
use crate::db::sqlite::platform_credential::SqlitePlatformCredentialRepository;
use crate::db::sqlite::queue::SqliteQueueRepository;
use crate::db::sqlite::test_pool;
use crate::platform::PlatformId;
use crate::random::StandartRandomProvider;
use crate::state::{
    AppConfigStore, AppState, AppStateBuilder, UniAppState, UniStateParams, assemble_uni_state,
};
use crate::widget_api;

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
    InMemoryPlatformCredentialRepository,
    InMemoryConfigRepository,
    InMemoryRuleRepository,
    InMemoryActionRepository,
    InMemoryGameOrderRepository,
    InMemoryMovieOrderRepository,
    InMemoryVipRecordRepository,
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
    let state = assemble_uni_state(UniStateParams {
        random: StandartRandomProvider,
        config: config_store,
        credentials_repo: Arc::new(InMemoryPlatformCredentialRepository::new()),
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

    let mut builder =
        AppStateBuilder::new(StandartRandomProvider, config_store, credentials_repo, pool);
    if !seeded {
        builder = builder.with_empty_repos();
    }
    if let Some(queue_repo) = queue_repo {
        builder = builder.with_queue_repo(queue_repo);
    }
    builder.build().await.expect("failed to build test state")
}

pub fn test_router(state: AppState) -> axum::Router {
    api::router(state.clone()).merge(widget_api::router(state))
}

pub fn api_path(path: &str) -> String {
    format!("/api{path}")
}

pub async fn save_twitch_credentials(state: &AppState, token: &str) {
    state
        .credentials
        .save_credential(PlatformId::TWITCH, token)
        .await
        .expect("failed to save twitch credentials")
}

pub async fn session_cookie(state: &AppState, twitch_id: &str) -> String {
    let app = test_router(state.clone());
    let ticket = state
        .session_service
        .create_login_ticket(twitch_id, Some("viewer"))
        .await
        .unwrap()
        .ticket
        .as_str()
        .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/sessions"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, format!("{LOGIN_COOKIE}={ticket}"))
                .body(Body::from(format!(r#"{{"ticket":"{ticket}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}
