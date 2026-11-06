#![cfg(test)]

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use axum::http::header;
use tower::ServiceExt;

use crate::api;
use crate::api::auth::LOGIN_COOKIE;
use crate::widget_api;
use composition::state::AppState;

pub use composition::test_fixtures::*;

pub fn test_router(state: AppState) -> axum::Router {
    api::router(state.clone()).merge(widget_api::router(state))
}

pub fn api_path(path: &str) -> String {
    format!("/api{path}")
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
