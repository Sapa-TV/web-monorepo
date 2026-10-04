use axum::body::Body;
use axum::http::header;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use crate::roulette::rarity::{Rarity, RarityId};
use crate::test_fixtures::{test_router, test_state};

#[tokio::test]
async fn rarities_are_read_only() {
    let state = test_state().await;
    let app = test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/wapi/rarities")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, "Bearer test-key")
                .body(Body::from(
                    r##"{"name":"x","display_name":"X","image":"x.png","color":"#fff"}"##,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn wak_key_can_list_rarities() {
    let state = test_state().await;
    state
        .rarity_service
        .save(Rarity::new(
            RarityId::new(0),
            "custom",
            "Custom",
            "c.png",
            "#fff",
        ))
        .await
        .unwrap();
    let app = test_router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/wapi/rarities")
                .header(header::AUTHORIZATION, "Bearer test-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}
