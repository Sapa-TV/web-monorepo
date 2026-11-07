use axum::body::Body;
use axum::body::to_bytes;
use axum::http::Request;
use axum::http::header;
use serde_json::Value;
use tower::ServiceExt;

use composition::config::static_config::StaticConfig;

use crate::test_fixtures::{api_path, test_router, test_state, test_state_with_static};

fn hook_request(token: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(api_path(&format!("/srs/hooks?token={token}")))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn hook_without_configured_secret_is_rejected() {
    let state = test_state().await;
    let app = test_router(state.clone());

    let response = app
        .oneshot(hook_request(
            "",
            r#"{"action":"on_publish","app":"live","stream":"stream1"}"#,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
    assert!(!state.stream_status.is_online());
}

#[tokio::test]
async fn publish_hook_brings_stream_online() {
    let state = test_state_with_static(StaticConfig::with_srs_hook_secret(Some(
        "secret".to_string(),
    )))
    .await;
    let app = test_router(state.clone());

    let response = app
        .oneshot(hook_request(
            "secret",
            r#"{"action":"on_publish","app":"live","stream":"stream1"}"#,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["code"], 0);
    assert!(state.stream_status.is_online());
}

#[tokio::test]
async fn unpublish_hook_takes_stream_offline() {
    let state = test_state_with_static(StaticConfig::with_srs_hook_secret(Some(
        "secret".to_string(),
    )))
    .await;
    state.stream_status.set_rtmp_online(true);
    let app = test_router(state.clone());

    let response = app
        .oneshot(hook_request(
            "secret",
            r#"{"action":"on_unpublish","app":"live","stream":"stream1"}"#,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), 200);
    assert!(!state.stream_status.is_online());
}

#[tokio::test]
async fn wrong_token_is_rejected() {
    let state = test_state_with_static(StaticConfig::with_srs_hook_secret(Some(
        "secret".to_string(),
    )))
    .await;
    let app = test_router(state.clone());

    let response = app
        .oneshot(hook_request(
            "wrong",
            r#"{"action":"on_publish","app":"live","stream":"stream1"}"#,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), 401);
    assert!(!state.stream_status.is_online());
}
