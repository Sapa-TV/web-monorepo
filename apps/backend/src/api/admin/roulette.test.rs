use axum::body::Body;
use axum::body::to_bytes;
use axum::http::header;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use crate::api::auth::SESSION_COOKIE;
use crate::roulette::rarity::Rarity;
use crate::roulette::rarity::RarityId;
use crate::roulette::slot_service::RouletteSlot;
use crate::roulette::slot_service::RouletteSlotId;
use crate::state::AppState;
use crate::test_fixtures::{api_path, session_cookie, test_router, test_state};

const COMMON: RarityId = RarityId::new(1);

/// Slot tests reference rarities 1 and 2 explicitly; the sqlite-backed
/// state has no seeded rarities, and FKs reject slots pointing at nothing.
async fn seed_common(state: &AppState) {
    for (id, name) in [(1u32, "common"), (2, "rare")] {
        state
            .rarity_service
            .save(Rarity::new(RarityId::new(id), name, name, "c.png", "#fff"))
            .await
            .unwrap();
    }
}

fn slot_body() -> &'static str {
    r#"{"name":"spin","rarity_id":1,"weight":10,"action":"enqueue_roulette"}"#
}

#[tokio::test]
async fn roulette_routes_require_admin_session_cookie() {
    let state = test_state().await;
    let app = test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/roulette/slots"))
                .header(header::COOKIE, format!("{SESSION_COOKIE}=bogus"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn admin_can_list_roulette_slots() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    seed_common(&state).await;
    state
        .slot_service
        .add_slot(RouletteSlot::new(
            RouletteSlotId::new(0),
            "spin",
            COMMON,
            10,
            "enqueue_roulette",
        ))
        .await
        .unwrap();

    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "123").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(api_path("/admin/roulette/slots"))
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
    assert_eq!(body[0]["rarity_id"], 1);
    assert_eq!(body[0]["weight"], 10);
}

#[tokio::test]
async fn admin_can_create_update_and_delete_slot() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    seed_common(&state).await;
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "123").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/admin/roulette/slots"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(slot_body()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let slot_id = body["id"].as_u64().unwrap();

    let updated = r#"{"name":"renamed","rarity_id":2,"weight":42,"action":"no_action"}"#;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(api_path(&format!("/admin/roulette/slots/{slot_id}")))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(updated))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["name"], "renamed");
    assert_eq!(body["weight"], 42);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(api_path(&format!("/admin/roulette/slots/{slot_id}")))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(state.slot_service.get_slots().is_empty());
}

#[tokio::test]
async fn delete_missing_slot_is_not_found() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "123").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(api_path("/admin/roulette/slots/999"))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_can_create_update_and_delete_rarity() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "123").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(api_path("/admin/roulette/rarities"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(
                    r##"{"name":"custom","display_name":"Custom","image":"c.png","color":"#fff"}"##,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    let rarity_id = body["id"].as_u64().unwrap();

    let updated = r##"{"name":"custom","display_name":"Renamed","image":"c.png","color":"#000"}"##;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(api_path(&format!("/admin/roulette/rarities/{rarity_id}")))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, &cookie)
                .body(Body::from(updated))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["display_name"], "Renamed");

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(api_path(&format!("/admin/roulette/rarities/{rarity_id}")))
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(state.rarity_service.get_all().is_empty());
}

#[tokio::test]
async fn update_missing_rarity_is_not_found() {
    let state = test_state().await;
    state.admin_service.add("123", None).await.unwrap();
    let app = test_router(state.clone());
    let cookie = session_cookie(&state, "123").await;

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(api_path("/admin/roulette/rarities/999"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(Body::from(
                    r##"{"name":"x","display_name":"X","image":"x.png","color":"#fff"}"##,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
