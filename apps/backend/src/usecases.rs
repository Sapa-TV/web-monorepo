// TODO: remove once every scenario module lands
#![allow(dead_code)]

use std::future::Future;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::body::to_bytes;
use axum::http::Method;
use axum::http::Request;
use axum::http::StatusCode;
use serde_json::Value;
use tokio::time::sleep;
use tower::ServiceExt;

pub(crate) const WAIT_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) type App = axum::Router;

pub(crate) fn bearer(key: &str) -> (&'static str, String) {
    ("authorization", format!("Bearer {key}"))
}

pub(crate) fn cookie(value: &str) -> (&'static str, String) {
    ("cookie", value.to_string())
}

pub(crate) async fn request_json(
    app: App,
    method: Method,
    uri: &str,
    headers: &[(&'static str, String)],
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(*name, value);
    }
    let request = builder
        .body(match body {
            Some(json) => Body::from(json.to_string()),
            None => Body::empty(),
        })
        .expect("valid request");

    let response = app.oneshot(request).await.expect("router response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, json)
}

pub(crate) async fn get_json(
    app: App,
    uri: &str,
    headers: &[(&'static str, String)],
) -> (StatusCode, Value) {
    request_json(app, Method::GET, uri, headers, None).await
}

pub(crate) async fn post_json(
    app: App,
    uri: &str,
    headers: &[(&'static str, String)],
    body: Value,
) -> (StatusCode, Value) {
    request_json(app, Method::POST, uri, headers, Some(body)).await
}

pub(crate) async fn patch_json(
    app: App,
    uri: &str,
    headers: &[(&'static str, String)],
    body: Value,
) -> (StatusCode, Value) {
    request_json(app, Method::PATCH, uri, headers, Some(body)).await
}

pub(crate) async fn wait_until<F, Fut>(timeout: Duration, mut predicate: F) -> bool
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let deadline = Instant::now() + timeout;
    loop {
        if predicate().await {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn full_router_responds() {
    use crate::test_fixtures::{api_path, test_router, test_state};

    let state = test_state().await;
    let app = test_router(state);
    let (status, _) = get_json(app, &api_path("/admin"), &[]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
