use std::slice::from_ref;
use std::time::Duration;

use axum::http::StatusCode;
use serde_json::json;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use super::WAIT_TIMEOUT;
use super::cookie;
use super::get_json;
use super::post_json;
use super::put_json;
use super::wait_until;
use super::wapi_path;
use crate::ingress::event::PlatformEvent;
use crate::platform::PlatformId;
use crate::runtime::start_rule_pipeline;
use crate::test_fixtures::api_path;
use crate::test_fixtures::session_cookie;
use crate::test_fixtures::test_router;
use crate::test_fixtures::test_state;

const WIDGET_KEY: &str = "test-key";

fn widget_auth() -> [(&'static str, String); 1] {
    [("authorization", format!("Bearer {WIDGET_KEY}"))]
}

fn chat_message(event_id: &str, user_id: &str, text: &str) -> PlatformEvent {
    PlatformEvent::chat_message(
        PlatformId::TWITCH,
        event_id,
        user_id.to_string(),
        format!("viewer-{user_id}"),
        text.to_string(),
    )
}

async fn queue_user_names(app: axum::Router) -> Vec<String> {
    let headers = widget_auth();
    let (status, page) = get_json(app, &wapi_path("/queue"), &headers).await;
    assert_eq!(status, StatusCode::OK);
    page["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|e| e["user_name"].as_str().expect("user_name").to_string())
        .collect()
}

#[tokio::test]
async fn chat_rule_triggers_action_and_fills_the_queue() {
    let state = test_state().await;
    state.admin_service.seed("100").await.unwrap();
    let admin_cookie = cookie(&session_cookie(&state, "100").await);
    let app = test_router(state.clone());

    start_rule_pipeline(&state, &CancellationToken::new());

    let (status, action) = post_json(
        app.clone(),
        &api_path("/admin/actions"),
        from_ref(&admin_cookie),
        json!({
            "name": "spin on !spin",
            "kind": { "type": "enqueue_roulette" },
            "enabled": true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let action_id = action["id"].as_i64().expect("action id");

    let rule_payload = json!({
        "name": "!spin rule",
        "enabled": true,
        "trigger": "chat_message",
        "conditions": {
            "trigger": "chat_message",
            "matcher": "equals",
            "pattern": "!spin"
        },
        "action_id": action_id
    });
    let (status, rule) = post_json(
        app.clone(),
        &api_path("/admin/rules"),
        from_ref(&admin_cookie),
        rule_payload.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let rule_id = rule["id"].as_i64().expect("rule id");

    state
        .ingress
        .publish(chat_message("msg-1", "u1", "!spin"))
        .await
        .unwrap();

    let matched = wait_until(WAIT_TIMEOUT, || {
        let app = app.clone();
        async move {
            queue_user_names(app)
                .await
                .iter()
                .any(|name| name == "viewer-u1")
        }
    })
    .await;
    assert!(
        matched,
        "matching message must enqueue the viewer via action"
    );

    state
        .ingress
        .publish(chat_message("msg-1", "u2", "!spin"))
        .await
        .unwrap();
    sleep(Duration::from_millis(50)).await;
    let names = queue_user_names(app.clone()).await;
    assert_eq!(
        names.iter().filter(|n| n.starts_with("viewer-u")).count(),
        1,
        "duplicate event_id must be dropped by ingress dedup"
    );

    state
        .ingress
        .publish(chat_message("msg-2", "u3", "!raffle"))
        .await
        .unwrap();
    sleep(Duration::from_millis(50)).await;
    assert!(
        !queue_user_names(app.clone())
            .await
            .iter()
            .any(|name| name == "viewer-u3"),
        "non-matching text must not trigger the rule"
    );

    let disabled_payload = json!({
        "name": "!spin rule",
        "enabled": false,
        "trigger": "chat_message",
        "conditions": {
            "trigger": "chat_message",
            "matcher": "equals",
            "pattern": "!spin"
        },
        "action_id": action_id
    });
    let update_uri = api_path(&format!("/admin/rules/{rule_id}"));
    let (status, _) = put_json(
        app.clone(),
        &update_uri,
        from_ref(&admin_cookie),
        disabled_payload,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    state
        .ingress
        .publish(chat_message("msg-3", "u4", "!spin"))
        .await
        .unwrap();
    sleep(Duration::from_millis(50)).await;
    assert!(
        !queue_user_names(app.clone())
            .await
            .iter()
            .any(|name| name == "viewer-u4"),
        "disabled rule must not trigger"
    );

    let headers = widget_auth();
    let (status, stats) = get_json(app, &wapi_path("/queue/stats"), &headers).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stats["pending"], 1);
}
