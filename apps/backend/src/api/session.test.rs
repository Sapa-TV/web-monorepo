use axum::body::Body;
use axum::body::to_bytes;
use axum::http::header;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use serde_json::Value;
use tower::ServiceExt;

use crate::api::auth::{LOGIN_COOKIE, SESSION_COOKIE};
use crate::state::AppState;
use crate::test_fixtures::{api_path, test_router, test_state};

async fn login_ticket(state: &AppState, twitch_id: &str) -> String {
    state
        .session_service
        .create_login_ticket(twitch_id, Some("viewer"))
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
    assert_eq!(body["twitch_user_id"], "123");
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
