use axum::body::{Body, to_bytes};
use axum::http::header;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::test_fixtures::{api_path, test_router, test_state};

async fn guest_cookie(app: &axum::Router) -> String {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/sessions/guest"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
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

#[tokio::test]
async fn config_is_public_and_disabled_without_keys() {
    let app = test_router(test_state().await);
    let response = app
        .oneshot(
            Request::builder()
                .uri(api_path("/push/config"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["enabled"], false);
    assert!(json["public_key"].is_null());
}

#[tokio::test]
async fn guest_can_subscribe_and_unsubscribe() {
    let state = test_state().await;
    let app = test_router(state.clone());
    let cookie = guest_cookie(&app).await;

    let subscription = serde_json::json!({
        "endpoint": "https://push.example/1",
        "keys": { "p256dh": "p256dh", "auth": "auth" }
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/push/subscriptions"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(subscription.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["endpoint"], "https://push.example/1");

    let token = cookie.split('=').nth(1).expect("token");
    let user_id = state
        .session_service
        .validate_session(token)
        .await
        .expect("session")
        .user_id;
    assert_eq!(
        state
            .push_service
            .subscriptions(user_id)
            .await
            .expect("subscriptions")
            .len(),
        1
    );

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(api_path("/push/subscriptions"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(Body::from(subscription.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(
        state
            .push_service
            .subscriptions(user_id)
            .await
            .expect("subscriptions")
            .is_empty()
    );
}

#[tokio::test]
async fn subscribe_requires_session() {
    let app = test_router(test_state().await);
    let subscription = serde_json::json!({
        "endpoint": "https://push.example/1",
        "keys": { "p256dh": "p256dh", "auth": "auth" }
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/push/subscriptions"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(subscription.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_endpoint_is_rejected() {
    let state = test_state().await;
    let app = test_router(state);
    let cookie = guest_cookie(&app).await;
    let subscription = serde_json::json!({
        "endpoint": "http://push.example/1",
        "keys": { "p256dh": "p256dh", "auth": "auth" }
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/push/subscriptions"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(Body::from(subscription.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
