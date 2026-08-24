use std::slice::from_ref;

use axum::http::StatusCode;
use serde_json::json;
use tokio::join;

use super::bearer;
use super::cookie;
use super::get_json;
use super::post_json;
use super::wapi_path;
use crate::test_fixtures::api_path;
use crate::test_fixtures::session_cookie;
use crate::test_fixtures::test_router;
use crate::test_fixtures::test_state;

const WIDGET_KEY: &str = "test-key";

fn widget_auth() -> [(&'static str, String); 1] {
    [bearer(WIDGET_KEY)]
}

#[tokio::test]
async fn operator_runs_full_roulette_cycle_over_http() {
    let state = test_state().await;
    state.admin_service.seed("100").await.unwrap();
    let admin_cookie = cookie(&session_cookie(&state, "100").await);
    let app = test_router(state.clone());

    let (status, rarity) = post_json(
        app.clone(),
        &api_path("/admin/roulette/rarities"),
        from_ref(&admin_cookie),
        json!({
            "name": "common",
            "display_name": "Common",
            "image": "common.png",
            "color": "#9d9d9d"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let rarity_id = rarity["id"].as_i64().expect("rarity id");

    let (status, slot) = post_json(
        app.clone(),
        &api_path("/admin/roulette/slots"),
        &[admin_cookie],
        json!({
            "name": "Джекпот",
            "rarity_id": rarity_id,
            "weight": 100,
            "action": "spin"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(slot["name"], "Джекпот");

    let headers = widget_auth();
    let (status, stats) = get_json(app.clone(), &wapi_path("/queue/stats"), &headers).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stats["pending"], 0);

    let headers = widget_auth();
    let (status, first) = post_json(
        app.clone(),
        &wapi_path("/queue/anonymous"),
        &headers,
        json!({ "name": "viewer1" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["status"], "Pending");
    assert_eq!(first["user_name"], "viewer1");
    let guest_user_id = first["user_id"].clone();

    let headers = widget_auth();
    let (status, second) = post_json(
        app.clone(),
        &wapi_path("/queue/anonymous"),
        &headers,
        json!({ "name": "viewer2" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        second["user_id"], guest_user_id,
        "anonymous viewers share the single guest user"
    );

    let headers = widget_auth();
    let (status, page) = get_json(app.clone(), &wapi_path("/queue"), &headers).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["entries"].as_array().expect("entries").len(), 2);

    let dequeue = |app: axum::Router| async move {
        let headers = widget_auth();
        post_json(app, &wapi_path("/queue/next"), &headers, json!({})).await
    };
    let ((status_a, body_a), (status_b, body_b)) =
        join!(dequeue(app.clone()), dequeue(app.clone()));
    let successes = [status_a, status_b]
        .iter()
        .filter(|s| **s == StatusCode::OK)
        .count();
    let conflicts = [status_a, status_b]
        .iter()
        .filter(|s| **s == StatusCode::CONFLICT)
        .count();
    assert_eq!(successes, 1, "exactly one spin may start");
    assert_eq!(conflicts, 1, "the second concurrent spin is rejected");

    let winner = if status_a == StatusCode::OK {
        body_a
    } else {
        body_b
    };
    assert_eq!(winner["entry"]["status"], "Spinning");
    assert_eq!(winner["slot"]["name"], "Джекпот");
    let spinning_entry_id = winner["entry"]["id"].as_i64().expect("entry id");

    let complete_uri = wapi_path(&format!("/queue/{spinning_entry_id}/complete"));
    let headers = widget_auth();
    let (status, _) = post_json(app.clone(), &complete_uri, &headers, json!({})).await;
    assert_eq!(status, StatusCode::OK);

    let headers = widget_auth();
    let (status, stats) = get_json(app.clone(), &wapi_path("/queue/stats"), &headers).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stats["completed"], 1);
    assert_eq!(stats["spinning"], 0);

    let headers = widget_auth();
    let (status, pending) = post_json(
        app.clone(),
        &wapi_path("/queue/anonymous"),
        &headers,
        json!({ "name": "viewer3" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let pending_entry_id = pending["id"].as_i64().expect("pending entry id");

    let cancel_uri = wapi_path(&format!("/queue/{pending_entry_id}/cancel"));
    let headers = widget_auth();
    let (status, _) = post_json(app.clone(), &cancel_uri, &headers, json!({})).await;
    assert_eq!(status, StatusCode::OK);

    let headers = widget_auth();
    let (status, stats) = get_json(app, &wapi_path("/queue/stats"), &headers).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stats["cancelled"], 1);
}
