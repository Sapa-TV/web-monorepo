use std::sync::Arc;

use axum::body::Body;
use axum::http::header;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::test_fixtures::{api_path, session_cookie, test_router, test_state};
use composition::config::static_config::StaticConfig;
use composition::config::store::ConfigStore;
use composition::config::twitch::TwitchConfig;
use composition::state::{AppState, AppStateBuilder};
use domain::random::StandartRandomProvider;
use model::config::runtime::RuntimeConfig;
use storage_sqlite::config::SqliteConfigRepository;
use storage_sqlite::platform_credential::SqlitePlatformCredentialRepository;
use storage_sqlite::test_pool;

fn twitch_config() -> Arc<TwitchConfig> {
    Arc::new(TwitchConfig::fixture())
}

async fn state_with_twitch() -> AppState {
    let static_cfg = StaticConfig::with_twitch(Some(twitch_config()));
    let (pool, _path) = test_pool().await;
    let config_store = Arc::new(ConfigStore::new(
        Arc::new(static_cfg),
        RuntimeConfig::test_runtime("test-key"),
        Arc::new(SqliteConfigRepository::new(pool.clone())),
    ));
    AppStateBuilder::new(
        StandartRandomProvider::new(),
        config_store,
        Arc::new(SqlitePlatformCredentialRepository::new(pool.clone())),
        pool,
    )
    .with_empty_repos()
    .build()
    .await
    .expect("failed to build test state")
}

#[tokio::test]
async fn find_user_requires_root_session() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "123").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/twitch/users?login=viewer"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn find_user_without_twitch_config_is_bad_request() {
    let state = test_state().await;
    state.admin_service.add("100", None).await.unwrap();
    state.admin_service.set_root("100", true).await.unwrap();
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "100").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/twitch/users?login=viewer"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn find_user_without_credentials_is_unauthorized() {
    let state = state_with_twitch().await;
    state.admin_service.add("100", None).await.unwrap();
    state.admin_service.set_root("100", true).await.unwrap();
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "100").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/twitch/users?login=viewer"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
