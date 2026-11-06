use axum::body::Body;
use axum::body::to_bytes;
use axum::http::Request;
use axum::http::StatusCode;
use axum::http::header;
use serde_json::json;
use tower::ServiceExt;

use super::cookie;
use super::get_json;
use super::post_json;
use crate::api::auth::LOGIN_COOKIE;
use crate::test_fixtures::api_path;
use crate::test_fixtures::test_router;
use crate::test_fixtures::test_state;

async fn create_session(app: axum::Router, ticket: &str) -> String {
    let request = Request::builder()
        .method("POST")
        .uri(api_path("/sessions"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, format!("{LOGIN_COOKIE}={ticket}"))
        .body(Body::from(format!(r#"{{"ticket":"{ticket}"}}"#)))
        .expect("valid request");
    let response = app.oneshot(request).await.expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body, usize::MAX).await.expect("response body");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["twitch_user_id"],
        "100"
    );
    parts
        .headers
        .get(header::SET_COOKIE)
        .expect("session cookie set")
        .to_str()
        .expect("cookie header str")
        .split(';')
        .next()
        .expect("cookie pair")
        .to_string()
}

async fn logout(app: axum::Router, session_cookie_pair: &str) -> StatusCode {
    let request = Request::builder()
        .method("DELETE")
        .uri(api_path("/sessions/me"))
        .header(header::COOKIE, session_cookie_pair)
        .body(Body::empty())
        .expect("valid request");
    let response = app.oneshot(request).await.expect("response");
    response.status()
}

#[tokio::test]
async fn session_lifecycle_from_ticket_to_logout() {
    let state = test_state().await;
    state.admin_service.seed("100").await.unwrap();
    let ticket = state
        .session_service
        .create_login_ticket("100", Some("root"))
        .await
        .unwrap()
        .ticket
        .as_str()
        .to_string();
    let app = test_router(state.clone());

    let (status, _) = get_json(app.clone(), &api_path("/admin"), &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let session_cookie_pair = create_session(app.clone(), &ticket).await;
    let session_headers = [cookie(&session_cookie_pair)];

    let (status, me) = get_json(app.clone(), &api_path("/sessions/me"), &session_headers).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["twitch_user_id"], "100");

    let (status, _) = get_json(app.clone(), &api_path("/admin"), &session_headers).await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = post_json(
        app.clone(),
        &api_path("/sessions"),
        &[],
        json!({ "ticket": ticket }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "tickets are one-shot");

    assert_eq!(
        logout(app.clone(), &session_cookie_pair).await,
        StatusCode::NO_CONTENT
    );

    let (status, _) = get_json(
        app.clone(),
        &api_path("/admin"),
        &[cookie(&session_cookie_pair)],
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    assert_eq!(
        logout(app, &session_cookie_pair).await,
        StatusCode::UNAUTHORIZED
    );
}
