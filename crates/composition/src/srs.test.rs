use super::*;

fn payload(action: &str, app: &str, stream: &str) -> SrsHookPayload {
    SrsHookPayload {
        action: action.to_string(),
        app: app.to_string(),
        stream: stream.to_string(),
    }
}

#[test]
fn publish_sets_rtmp_online() {
    let status = StreamStatus::new();
    let outcome = handle_hook(
        &status,
        Some("secret"),
        "secret",
        &payload("on_publish", "live", "stream1"),
    );

    assert_eq!(outcome.status, 200);
    assert_eq!(outcome.code, 0);
    assert!(status.is_online());
}

#[test]
fn unpublish_sets_rtmp_offline() {
    let status = StreamStatus::new();
    status.set_rtmp_online(true);
    handle_hook(
        &status,
        Some("secret"),
        "secret",
        &payload("on_unpublish", "live", "stream1"),
    );

    assert!(!status.is_online());
}

#[test]
fn unknown_action_keeps_status() {
    let status = StreamStatus::new();
    status.set_rtmp_online(true);
    handle_hook(
        &status,
        Some("secret"),
        "secret",
        &payload("on_play", "live", "stream1"),
    );

    assert!(status.is_online());
}

#[test]
fn other_streams_are_ignored() {
    let status = StreamStatus::new();
    handle_hook(
        &status,
        Some("secret"),
        "secret",
        &payload("on_publish", "live", "other"),
    );
    handle_hook(
        &status,
        Some("secret"),
        "secret",
        &payload("on_publish", "other", "stream1"),
    );

    assert!(!status.is_online());
}

#[test]
fn missing_secret_rejects_every_hook() {
    let status = StreamStatus::new();
    let outcome = handle_hook(&status, None, "", &payload("on_publish", "live", "stream1"));

    assert_eq!(outcome.status, 401);
    assert!(!status.is_online());
}

#[test]
fn wrong_token_is_rejected() {
    let status = StreamStatus::new();
    let outcome = handle_hook(
        &status,
        Some("secret"),
        "wrong",
        &payload("on_publish", "live", "stream1"),
    );

    assert_eq!(outcome.status, 401);
    assert!(!status.is_online());
}

#[test]
fn streams_online_finds_tracked_publish() {
    let body: SrsStreamsResponse = serde_json::from_str(
        r#"{"streams":[
            {"name":"stream1","app":"live","publish":{"active":true}},
            {"name":"other","app":"live","publish":{"active":false}}
        ]}"#,
    )
    .unwrap();

    assert!(streams_online(&body));
}

#[test]
fn streams_online_ignores_inactive_and_foreign_streams() {
    let body: SrsStreamsResponse = serde_json::from_str(
        r#"{"streams":[
            {"name":"stream1","app":"live","publish":{"active":false}},
            {"name":"stream1","app":"other"}
        ]}"#,
    )
    .unwrap();

    assert!(!streams_online(&body));
}

#[test]
fn streams_online_handles_empty_response() {
    let body: SrsStreamsResponse = serde_json::from_str(r#"{"code":0,"streams":[]}"#).unwrap();

    assert!(!streams_online(&body));
}
