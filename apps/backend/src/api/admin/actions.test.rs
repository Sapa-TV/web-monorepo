use axum::body::Body;
use axum::body::to_bytes;
use axum::http::header;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use crate::actions::action::ActionKind;
use crate::api::auth::SESSION_COOKIE;
use crate::test_fixtures::{api_path, session_cookie, test_router, test_state};

fn action_body() -> &'static str {
    r#"{"name":"spin","kind":{"type":"enqueue_roulette"},"enabled":true}"#
}

#[tokio::test]
async fn actions_require_session() {
    let state = test_state().await;
    let app = test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/actions"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_can_list_actions() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    state
        .action_service
        .create("spin", ActionKind::EnqueueRoulette, true)
        .await
        .unwrap();

    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "123").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/actions"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["name"], "spin");
    assert_eq!(body[0]["kind"]["type"], "enqueue_roulette");
}

#[tokio::test]
async fn only_root_can_mutate_actions() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    state.admin_service.add("100", None).await.unwrap();
    state.admin_service.set_root("100", true).await.unwrap();
    let app = test_router(state.clone());

    let user_cookie = session_cookie(&state, "123").await;
    let root_cookie = session_cookie(&state, "100").await;

    let create = |cookie: String| {
        app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/admin/actions"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(Body::from(action_body()))
                .unwrap(),
        )
    };

    let response = create(user_cookie).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = create(root_cookie).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(state.action_service.list().await.unwrap().len(), 1);
}

#[tokio::test]
async fn root_can_update_and_delete_action() {
    let state = test_state().await;
    state.admin_service.add("100", None).await.unwrap();
    state.admin_service.set_root("100", true).await.unwrap();
    let app = test_router(state.clone());
    let root_cookie = session_cookie(&state, "100").await;
    let action = state
        .action_service
        .create("spin", ActionKind::EnqueueRoulette, true)
        .await
        .unwrap();
    let action_id = action.id.get();

    let updated = r#"{"name":"renamed","kind":{"type":"no_action"},"enabled":false}"#;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(api_path(&format!("/admin/actions/{action_id}")))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &root_cookie)
                .body(Body::from(updated))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["name"], "renamed");
    assert_eq!(body["enabled"], false);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(api_path(&format!("/admin/actions/{action_id}")))
                .header(header::COOKIE, root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(state.action_service.get(action.id).await.unwrap().is_none());
}

#[tokio::test]
async fn delete_missing_action_is_not_found() {
    let state = test_state().await;
    state.admin_service.add("100", None).await.unwrap();
    state.admin_service.set_root("100", true).await.unwrap();
    let app = test_router(state.clone());
    let root_cookie = session_cookie(&state, "100").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(api_path("/admin/actions/999"))
                .header(header::COOKIE, root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn actions_require_admin_session_cookie() {
    let state = test_state().await;
    let app = test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/actions"))
                .header(header::COOKIE, format!("{SESSION_COOKIE}=bogus"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
