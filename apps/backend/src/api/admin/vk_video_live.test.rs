use std::sync::Arc;

use axum::body::Body;
use axum::body::to_bytes;
use axum::http::header;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::config::runtime::RuntimeConfig;
use crate::config::static_config::StaticConfig;
use crate::config::store::ConfigStore;
use crate::config::vk_video_live::VkVideoLiveConfig;
use crate::db::sqlite::config::SqliteConfigRepository;
use crate::db::sqlite::platform_credential::SqlitePlatformCredentialRepository;
use crate::db::sqlite::test_pool;
use crate::random::StandartRandomProvider;
use crate::state::{AppState, AppStateBuilder};
use crate::test_fixtures::{api_path, session_cookie, test_router, test_state};

async fn state_with_vk() -> AppState {
    let static_cfg = StaticConfig::with_vk_video_live(Some(Arc::new(VkVideoLiveConfig::fixture())));
    let (pool, _path) = test_pool().await;
    let config_store = Arc::new(ConfigStore::new(
        Arc::new(static_cfg),
        RuntimeConfig::test_runtime("test-key"),
        Arc::new(SqliteConfigRepository::new(pool.clone())),
    ));
    AppStateBuilder::new(
        StandartRandomProvider,
        config_store,
        Arc::new(SqlitePlatformCredentialRepository::new(pool.clone())),
        pool,
    )
    .with_empty_repos()
    .build()
    .await
    .expect("failed to build test state")
}

async fn root_session_state() -> AppState {
    let state = state_with_vk().await;
    state.admin_service.add("100", None).await.unwrap();
    state.admin_service.set_root("100", true).await.unwrap();
    state
}

#[tokio::test]
async fn start_requires_root_session() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "123").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/vk-video-live/auth"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn start_without_config_is_bad_request() {
    let state = test_state().await;
    state.admin_service.add("100", None).await.unwrap();
    state.admin_service.set_root("100", true).await.unwrap();
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "100").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/vk-video-live/auth"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn start_returns_authorize_url() {
    let state = root_session_state().await;
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "100").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/vk-video-live/auth"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        value["auth_url"]
            .as_str()
            .expect("auth_url present")
            .starts_with("https://auth.live.vkvideo.ru/app/oauth2/authorize")
    );
}

#[tokio::test]
async fn callback_with_unknown_state_is_forbidden() {
    let state = root_session_state().await;
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "100").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path(
                    "/admin/vk-video-live/auth/callback?code=abc&state=unknown",
                ))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn credentials_status_lists_platforms() {
    let state = root_session_state().await;
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "100").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/ingress/credentials"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["twitch"], false);
    assert_eq!(value["vk_video_live"], false);
}
