use axum::body::Body;
use axum::body::to_bytes;
use axum::http::header;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use serde_json::Value;
use tower::ServiceExt;

use composition::state::AppState;
use model::platform::PlatformId;

use crate::api::auth::{LOGIN_COOKIE, SESSION_COOKIE};
use crate::test_fixtures::{api_path, test_router, test_state};

async fn login_ticket(state: &AppState, twitch_id: &str) -> String {
    state
        .session_service
        .create_login_ticket(PlatformId::TWITCH, twitch_id, Some("viewer"))
        .await
        .unwrap()
        .ticket
        .as_str()
        .to_string()
}

async fn create_session(state: &AppState, twitch_id: &str) -> Response {
    let app = test_router(state.clone());
    let ticket = login_ticket(state, twitch_id).await;

    app.oneshot(
        Request::builder()
            .method("POST")
            .uri(api_path("/sessions"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, format!("{LOGIN_COOKIE}={ticket}"))
            .body(Body::from(format!(r#"{{"ticket":"{ticket}"}}"#)))
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn session_cookie(state: &AppState, twitch_id: &str) -> String {
    let response = create_session(state, twitch_id).await;
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

#[tokio::test]
async fn me_requires_session() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    let app = test_router(state.clone());

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/sessions/me"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let cookie = session_cookie(&state, "123").await;
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/sessions/me"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["platform_user_id"], "123");
    assert_eq!(body["is_guest"], false);
    assert!(body["user_id"].is_number());
}

#[tokio::test]
async fn guest_session_creates_identity_and_reuses_it() {
    let state = test_state().await;
    let app = test_router(state);

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
    let cookie = response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["is_guest"], true);
    assert_eq!(body["platform_user_id"], Value::Null);
    let user_id = body["user_id"].clone();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/sessions/me"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["user_id"], user_id);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/sessions/guest"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["user_id"], user_id);
}

#[tokio::test]
async fn link_adds_new_provider_to_current_account() {
    let state = test_state().await;
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "alice").await;
    let ticket = state
        .session_service
        .create_login_ticket(PlatformId::VK, "vk-alice", Some("Alice VK"))
        .await
        .unwrap()
        .ticket
        .as_str()
        .to_string();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/sessions/link"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, format!("{cookie}; {LOGIN_COOKIE}={ticket}"))
                .body(Body::from(format!(
                    r#"{{"ticket":"{ticket}","merge":false}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["platform"], "vk");
    assert_eq!(body["platform_user_id"], "vk-alice");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/sessions/me/links"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let links: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let platforms: Vec<&str> = links
        .as_array()
        .unwrap()
        .iter()
        .map(|link| link["platform"].as_str().unwrap())
        .collect();
    assert!(platforms.contains(&"twitch"));
    assert!(platforms.contains(&"vk"));
}

#[tokio::test]
async fn link_conflict_requires_merge_and_keeps_current_account() {
    let state = test_state().await;
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "alice").await;
    state
        .user_service
        .ensure_user_by_platform("vk", "vk-bob", "Bob")
        .await
        .unwrap();
    let ticket = state
        .session_service
        .create_login_ticket(PlatformId::VK, "vk-bob", Some("Bob"))
        .await
        .unwrap()
        .ticket
        .as_str()
        .to_string();
    let request = |merge: bool| {
        Request::builder()
            .method("POST")
            .uri(api_path("/sessions/link"))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, format!("{cookie}; {LOGIN_COOKIE}={ticket}"))
            .body(Body::from(format!(
                r#"{{"ticket":"{ticket}","merge":{merge}}}"#
            )))
            .unwrap()
    };

    let response = app.clone().oneshot(request(false)).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let response = app.clone().oneshot(request(true)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["platform_user_id"], "vk-bob");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/sessions/me/links"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let links: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let platforms: Vec<&str> = links
        .as_array()
        .unwrap()
        .iter()
        .map(|link| link["platform"].as_str().unwrap())
        .collect();
    assert!(platforms.contains(&"twitch"));
    assert!(platforms.contains(&"vk"));
}

#[tokio::test]
async fn me_works_for_regular_user() {
    let state = test_state().await;
    let app = test_router(state.clone());

    let cookie = session_cookie(&state, "999").await;
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/sessions/me"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn invalid_session_cookie_is_rejected() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    let app = test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/sessions/me"))
                .header(header::COOKIE, format!("{SESSION_COOKIE}=bogus"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn logout_destroys_session() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "123").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(api_path("/sessions/me"))
                .header(header::COOKIE, cookie.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/sessions/me"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn guest_sessions_are_rate_limited_per_address() {
    let state = test_state().await;
    let app = test_router(state);

    for _ in 0..5 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(api_path("/sessions/guest"))
                    .header("x-real-ip", "203.0.113.7")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/sessions/guest"))
                .header("x-real-ip", "203.0.113.7")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
}
