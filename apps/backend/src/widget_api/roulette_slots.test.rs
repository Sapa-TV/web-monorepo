use axum::body::Body;
use axum::http::header;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::roulette::rarity::{Rarity, RarityId};
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
use crate::test_fixtures::{test_router, test_state};

#[tokio::test]
async fn slots_are_read_only() {
    let state = test_state().await;
    let app = test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/wapi/slots")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer test-key")
                .body(Body::from(
                    r#"{"name":"x","rarity_id":1,"weight":1,"action":""}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn wak_key_can_list_slots() {
    let state = test_state().await;
    state
        .rarity_service
        .save(Rarity::new(
            RarityId::new(1),
            "common",
            "Common",
            "c.png",
            "#fff",
        ))
        .await
        .unwrap();
    state
        .slot_service
        .add_slot(RouletteSlot::new(
            RouletteSlotId::new(0),
            "spin",
            RarityId::new(1),
            10,
            "enqueue_roulette",
        ))
        .await
        .unwrap();
    let app = test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/wapi/slots")
                .header(header::AUTHORIZATION, "Bearer test-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
