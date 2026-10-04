use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use crate::orders::game::{GameOrderKind, NewGameOrder, OrderSource};
use crate::orders::movie::{MovieKind, NewMovieOrder};
use crate::orders::status::OrderStatus;
use crate::test_fixtures::{api_path, test_router, test_state};

async fn get_status(app: &axum::Router, uri: String) -> StatusCode {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    response.status()
}

async fn get_json(app: &axum::Router, uri: String) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    (status, body)
}

fn game(
    title: Option<&str>,
    customer: &str,
    kind: GameOrderKind,
    source: OrderSource,
    status: OrderStatus,
) -> NewGameOrder {
    NewGameOrder::new(
        title.map(str::to_string),
        customer.to_string(),
        None,
        kind,
        source,
        status,
        None,
        None,
    )
}

#[tokio::test]
async fn game_orders_are_public_and_filterable() {
    let state = test_state().await;
    state
        .game_order_service
        .create(game(
            Some("Test Game"),
            "user_one",
            GameOrderKind::Stream,
            OrderSource::Roulette,
            OrderStatus::Pending,
        ))
        .await
        .unwrap();
    state
        .game_order_service
        .create(game(
            Some("Strategy Game"),
            "user_two",
            GameOrderKind::Stream,
            OrderSource::Donate,
            OrderStatus::Completed,
        ))
        .await
        .unwrap();
    state
        .game_order_service
        .create(game(
            None,
            "user_three",
            GameOrderKind::Playthrough,
            OrderSource::Roulette,
            OrderStatus::Pending,
        ))
        .await
        .unwrap();
    let app = test_router(state);

    let (status, body) = get_json(&app, api_path("/orders/games")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 3);
    assert_eq!(body[0]["customer_name"], "user_three");
    assert_eq!(body[0]["title"], Value::Null);
    assert_eq!(body[2]["customer_name"], "user_one");

    let (_, body) = get_json(&app, api_path("/orders/games?status=pending")).await;
    assert_eq!(body.as_array().unwrap().len(), 2);

    let (_, body) = get_json(&app, api_path("/orders/games?kind=playthrough")).await;
    assert_eq!(body.as_array().unwrap().len(), 1);

    let (_, body) = get_json(
        &app,
        api_path("/orders/games?source=roulette&status=pending"),
    )
    .await;
    assert_eq!(body.as_array().unwrap().len(), 2);

    let (_, body) = get_json(&app, api_path("/orders/games?q=strateg")).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["customer_name"], "user_two");

    let (_, body) = get_json(&app, api_path("/orders/games?q=USER_THREE")).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn movie_orders_are_public_and_filterable() {
    let state = test_state().await;
    state
        .movie_order_service
        .create(NewMovieOrder::new(
            Some("Тестовый фильм".to_string()),
            "user_four".to_string(),
            None,
            MovieKind::Movie,
            OrderSource::Donate,
            OrderStatus::Pending,
            None,
        ))
        .await
        .unwrap();
    state
        .movie_order_service
        .create(NewMovieOrder::new(
            Some("Аниме тест".to_string()),
            "user_three".to_string(),
            None,
            MovieKind::Anime,
            OrderSource::Roulette,
            OrderStatus::Completed,
            None,
        ))
        .await
        .unwrap();
    let app = test_router(state);

    let (status, body) = get_json(&app, api_path("/orders/movies")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 2);

    let (_, body) = get_json(&app, api_path("/orders/movies?kind=anime")).await;
    assert_eq!(body.as_array().unwrap().len(), 1);

    let (_, body) = get_json(&app, api_path("/orders/movies?status=completed")).await;
    assert_eq!(body.as_array().unwrap().len(), 1);

    let (_, body) = get_json(
        &app,
        api_path("/orders/movies?q=%D1%82%D0%B5%D1%81%D1%82%D0%BE%D0%B2%D1%8B%D0%B9"),
    )
    .await;
    assert_eq!(body.as_array().unwrap().len(), 1);
    assert_eq!(body[0]["kind"], "movie");
}

#[tokio::test]
async fn orders_reject_unknown_filter_values() {
    let state = test_state().await;
    let app = test_router(state);

    let status = get_status(&app, api_path("/orders/games?status=bogus")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let status = get_status(&app, api_path("/orders/movies?kind=bogus")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
