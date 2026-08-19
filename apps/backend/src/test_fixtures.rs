#![cfg(test)]

use std::sync::Arc;

use axum::body::Body;
use axum::http::header;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::api;
use crate::api::auth::LOGIN_COOKIE;
use crate::config::runtime::RuntimeConfig;
use crate::config::static_config::StaticConfig;
use crate::config::store::ConfigStore;
use crate::db::inmemory_config::InMemoryConfigRepository;
use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
use crate::db::inmemory_queue::InMemoryQueueRepository;
use crate::platform::PlatformId;
use crate::random::StandartRandomProvider;
use crate::state::{AppState, AppStateBuilder};
use crate::widget_api;

pub async fn test_state() -> AppState {
    test_state_with_data(
        Arc::new(InMemoryQueueRepository::new()),
        Arc::new(InMemoryConfigRepository::new()),
    )
    .await
}

pub fn test_router(state: AppState) -> axum::Router {
    api::router(state.clone()).merge(widget_api::router(state))
}

pub fn api_path(path: &str) -> String {
    format!("/api{path}")
}

pub async fn test_state_with_data(
    queue_repo: Arc<InMemoryQueueRepository>,
    config_repo: Arc<InMemoryConfigRepository>,
) -> AppState {
    let config_store = Arc::new(ConfigStore::new(
        Arc::new(StaticConfig::test_config()),
        RuntimeConfig::test_runtime("test-key"),
        config_repo,
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
