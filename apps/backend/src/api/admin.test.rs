use std::sync::Arc;

use axum::body::Body;
use axum::body::to_bytes;
use axum::http::header;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use crate::api::auth::SESSION_COOKIE;
use crate::config::repository::ConfigRepository;
use crate::db::sqlite::config::SqliteConfigRepository;
use crate::db::sqlite::queue::SqliteQueueRepository;
use crate::db::sqlite::test_pool;
use crate::presence::WsClientRole;
use crate::test_fixtures::{api_path, session_cookie, test_router, test_state, test_state_with};

#[tokio::test]
async fn admin_routes_require_session() {
    let state = test_state().await;
    let app = test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/widget-access-key"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn regular_user_is_forbidden_from_admin_routes() {
    let state = test_state().await;
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "999").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/widget-access-key"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_can_read_widget_access_key_and_list_admins() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "123").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/widget-access-key"))
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["widget_access_key"], "test-key");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn presence_reflects_connected_ws_clients() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "123").await;

    let get_presence = |cookie: String, app: axum::Router| {
        app.oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/presence"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
    };

    let response = get_presence(cookie.clone(), app.clone()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["dock_connected"], false);
    assert_eq!(body["widget_count"], 0);

    {
        let _widget = state.presence.add(WsClientRole::Widget);
        let _dock = state.presence.add(WsClientRole::Dock);

        let response = get_presence(cookie, app).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["dock_connected"], true);
        assert_eq!(body["widget_count"], 1);
    }
}

#[tokio::test]
async fn only_root_can_add_admin() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    state.admin_service.add("100", None).await.unwrap();
    state.admin_service.set_root("100", true).await.unwrap();
    let app = test_router(state.clone());

    let user_cookie = session_cookie(&state, "123").await;
    let root_cookie = session_cookie(&state, "100").await;

    let add = |cookie: String| {
        app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/admin"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(Body::from(r#"{"twitch_id":"200","display_name":"mod"}"#))
                .unwrap(),
        )
    };

    let response = add(user_cookie).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = add(root_cookie).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert!(state.admin_service.is_admin("200").await.unwrap());
}

#[tokio::test]
async fn only_root_can_remove_admin() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    state.admin_service.add("100", None).await.unwrap();
    state.admin_service.set_root("100", true).await.unwrap();
    state.admin_service.add("200", None).await.unwrap();
    let app = test_router(state.clone());

    let user_cookie = session_cookie(&state, "123").await;
    let root_cookie = session_cookie(&state, "100").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(api_path("/admin/200"))
                .header(header::COOKIE, user_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(api_path("/admin/200"))
                .header(header::COOKIE, root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(!state.admin_service.is_admin("200").await.unwrap());
}

#[tokio::test]
async fn root_cannot_remove_self() {
    let state = test_state().await;
    state.admin_service.add("100", None).await.unwrap();
    state.admin_service.set_root("100", true).await.unwrap();
    let app = test_router(state.clone());
    let root_cookie = session_cookie(&state, "100").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(api_path("/admin/100"))
                .header(header::COOKIE, root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(state.admin_service.is_admin("100").await.unwrap());
}

#[tokio::test]
async fn regular_session_cookie_is_rejected_by_admin_middleware() {
    let state = test_state().await;
    let app = test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/widget-access-key"))
                .header(header::COOKIE, format!("{SESSION_COOKIE}=bogus"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn only_root_can_rotate_widget_access_key() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    state.admin_service.add("100", None).await.unwrap();
    state.admin_service.set_root("100", true).await.unwrap();
    let app = test_router(state.clone());

    let user_cookie = session_cookie(&state, "123").await;
    let root_cookie = session_cookie(&state, "100").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/admin/widget-access-key"))
                .header(header::COOKIE, user_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/admin/widget-access-key"))
                .header(header::COOKIE, root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let widget_access_key = body["widget_access_key"].as_str().unwrap().to_string();
    assert!(!widget_access_key.is_empty());
    assert_ne!(widget_access_key, "test-key");
    assert_eq!(state.config.widget_access_key(), widget_access_key);
}

#[tokio::test]
async fn rotate_widget_access_key_is_persisted_to_repo() {
    let (pool, _path) = test_pool().await;
    let config_repo = Arc::new(SqliteConfigRepository::new(pool.clone()));
    let queue_repo = Arc::new(SqliteQueueRepository::new(pool.clone()));
    let state = test_state_with(pool, Some(queue_repo)).await;
    state.admin_service.add("100", None).await.unwrap();
    state.admin_service.set_root("100", true).await.unwrap();
    let app = test_router(state.clone());
    let root_cookie = session_cookie(&state, "100").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/admin/widget-access-key"))
                .header(header::COOKIE, root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let widget_access_key = body["widget_access_key"].as_str().unwrap().to_string();

    let stored = config_repo.load().await.unwrap().unwrap();
    assert_eq!(stored.widget_access_key, widget_access_key);
}

#[tokio::test]
async fn rotated_widget_access_key_is_returned_by_get() {
    let state = test_state().await;
    state.admin_service.add("100", None).await.unwrap();
    state.admin_service.set_root("100", true).await.unwrap();
    let app = test_router(state.clone());
    let root_cookie = session_cookie(&state, "100").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/admin/widget-access-key"))
                .header(header::COOKIE, &root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let widget_access_key = body["widget_access_key"].as_str().unwrap().to_string();
    assert_ne!(widget_access_key, "test-key");
    assert_eq!(state.config.widget_access_key(), widget_access_key);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/widget-access-key"))
                .header(header::COOKIE, root_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["widget_access_key"], widget_access_key);
}
