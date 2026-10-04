use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;

use crate::state::AppState;
use crate::test_fixtures::{api_path, session_cookie, test_router, test_state};

async fn admin_cookie(state: &AppState) -> String {
    state.admin_service.add("123", None).await.unwrap();
    session_cookie(state, "123").await
}

async fn request(
    app: &axum::Router,
    method: &str,
    uri: String,
    cookie: Option<&str>,
    body: Option<String>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
    let body = match body {
        Some(json) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(json)
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

#[tokio::test]
async fn order_routes_require_admin() {
    let state = test_state().await;
    let app = test_router(state.clone());

    let (status, _) = request(&app, "GET", api_path("/admin/orders/vip"), None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let cookie = session_cookie(&state, "999").await;
    let (status, _) = request(
        &app,
        "GET",
        api_path("/admin/orders/vip"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_can_crud_game_orders() {
    let state = test_state().await;
    let app = test_router(state.clone());
    let cookie = admin_cookie(&state).await;

    let (status, body) = request(
        &app,
        "POST",
        api_path("/admin/orders/games"),
        Some(&cookie),
        Some(r#"{"title":null,"customer_name":"user_three","kind":"stream","source":"roulette","status":"pending","completed_at":null,"comment":null}"#.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let id = body["id"].as_u64().unwrap();
    assert_eq!(body["title"], Value::Null);
    assert_eq!(body["customer_name"], "user_three");

    let (status, body) = request(
        &app,
        "PUT",
        api_path(&format!("/admin/orders/games/{id}")),
        Some(&cookie),
        Some(r#"{"title":"Test Game","customer_name":"user_three","kind":"stream","source":"roulette","status":"completed","completed_at":"2026-10-04","comment":"done"}"#.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["title"], "Test Game");
    assert_eq!(body["status"], "completed");
    assert_eq!(body["completed_at"], "2026-10-04");

    let (status, _) = request(
        &app,
        "DELETE",
        api_path(&format!("/admin/orders/games/{id}")),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = request(
        &app,
        "PUT",
        api_path(&format!("/admin/orders/games/{id}")),
        Some(&cookie),
        Some(r#"{"title":null,"customer_name":"x","kind":"stream","source":"other","status":"pending","completed_at":null,"comment":null}"#.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn admin_can_crud_movie_orders() {
    let state = test_state().await;
    let app = test_router(state.clone());
    let cookie = admin_cookie(&state).await;

    let (status, body) = request(
        &app,
        "POST",
        api_path("/admin/orders/movies"),
        Some(&cookie),
        Some(r#"{"title":"Фильм тест","customer_name":"Тест Заказчиков","kind":"movie","source":"donate","status":"pending","comment":null}"#.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let id = body["id"].as_u64().unwrap();
    assert_eq!(body["kind"], "movie");

    let (status, body) = request(
        &app,
        "PUT",
        api_path(&format!("/admin/orders/movies/{id}")),
        Some(&cookie),
        Some(r#"{"title":"Фильм тест","customer_name":"Тест Заказчиков","kind":"movie","source":"donate","status":"completed","comment":"ok"}"#.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "completed");

    let (status, _) = request(
        &app,
        "DELETE",
        api_path(&format!("/admin/orders/movies/{id}")),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn admin_can_crud_vip_records_with_default_end_date() {
    let state = test_state().await;
    let app = test_router(state.clone());
    let cookie = admin_cookie(&state).await;

    let (status, body) = request(
        &app,
        "POST",
        api_path("/admin/orders/vip"),
        Some(&cookie),
        Some(r#"{"customer_name":"vip_user","kind":"vip","roulette_date":"2026-10-01","end_date":null,"note":null}"#.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let id = body["id"].as_u64().unwrap();
    assert_eq!(body["end_date"], "2026-10-15");
    assert_eq!(body["status"], "active");

    let (status, body) = request(
        &app,
        "PUT",
        api_path(&format!("/admin/orders/vip/{id}")),
        Some(&cookie),
        Some(r#"{"customer_name":"vip_user","kind":"vip","roulette_date":"2026-10-01","end_date":"2026-10-15","status":"done","note":null}"#.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "done");

    let (status, _) = request(
        &app,
        "POST",
        api_path("/admin/orders/vip"),
        Some(&cookie),
        Some(r#"{"customer_name":"x","kind":"unvip","roulette_date":"2026-10-10","end_date":"2026-10-01","note":null}"#.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, body) = request(
        &app,
        "GET",
        api_path("/admin/orders/vip"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.as_array().unwrap().len(), 1);

    let (status, _) = request(
        &app,
        "DELETE",
        api_path(&format!("/admin/orders/vip/{id}")),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn vip_reminders_reflect_dates() {
    let state = test_state().await;
    let app = test_router(state.clone());
    let cookie = admin_cookie(&state).await;

    let today = chrono::Utc::now().date_naive();
    let soon = today + chrono::Duration::days(2);
    let past = today - chrono::Duration::days(1);
    let start = today - chrono::Duration::days(10);

    let body = format!(
        r#"{{"customer_name":"soon","kind":"vip","roulette_date":"{start}","end_date":"{soon}","note":null}}"#
    );
    request(
        &app,
        "POST",
        api_path("/admin/orders/vip"),
        Some(&cookie),
        Some(body),
    )
    .await;

    let body = format!(
        r#"{{"customer_name":"loser","kind":"unvip","roulette_date":"{start}","end_date":"{past}","note":null}}"#
    );
    request(
        &app,
        "POST",
        api_path("/admin/orders/vip"),
        Some(&cookie),
        Some(body),
    )
    .await;

    let (status, body) = request(
        &app,
        "GET",
        api_path("/admin/orders/vip/reminders?days=3"),
        Some(&cookie),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["expiring"].as_array().unwrap().len(), 1);
    assert_eq!(body["expiring"][0]["customer_name"], "soon");
    assert_eq!(body["awaiting_return"].as_array().unwrap().len(), 1);
    assert_eq!(body["awaiting_return"][0]["customer_name"], "loser");
}
