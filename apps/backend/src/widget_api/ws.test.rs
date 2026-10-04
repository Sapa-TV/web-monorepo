use serde_json::Value;

use crate::presence::{PresenceSnapshot, WsClientRole};
use crate::queue::entry::{QueueEntryId, QueueStatus};
use crate::roulette::rarity::{Rarity, RarityId};
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
use crate::state::AppState;
use crate::test_fixtures::test_state;
use crate::widget_api::ws::{ClientMessage, ServerMessage, handle_message};

async fn setup_spinning(state: &AppState) -> QueueEntryId {
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
            "test_slot",
            RarityId::new(1),
            100,
            "test",
        ))
        .await
        .unwrap();
    let user_id = state.user_service.create("user1").await.unwrap().id;
    state.queue_service.enqueue(user_id, "user1").await.unwrap();
    let (entry, _slot) = state.queue_service.dequeue_next().await.unwrap();
    entry.id
}

#[tokio::test]
async fn auth_handshake_validates_token() {
    let state = test_state().await;

    let ok = handle_message(
        &state,
        ClientMessage::Auth {
            token: "test-key".to_string(),
            role: WsClientRole::Dock,
        },
    )
    .await;
    assert!(matches!(ok, ServerMessage::AuthOk));

    let bad = handle_message(
        &state,
        ClientMessage::Auth {
            token: "wrong-key".to_string(),
            role: WsClientRole::Widget,
        },
    )
    .await;
    assert!(matches!(bad, ServerMessage::AuthErr));
}

#[test]
fn auth_without_role_is_rejected_by_deserialization() {
    let parsed = serde_json::from_str::<ClientMessage>(r#"{"type":"auth","token":"k"}"#);
    assert!(parsed.is_err(), "auth requires a role field");

    let parsed =
        serde_json::from_str::<ClientMessage>(r#"{"type":"auth","token":"k","role":"widget"}"#);
    assert!(parsed.is_ok());
}

#[tokio::test]
async fn presence_snapshot_serializes_with_type_tag() {
    let msg = ServerMessage::presence(PresenceSnapshot::new(2, 3));
    let json: Value = serde_json::to_value(&msg).unwrap();
    assert_eq!(json["type"], "presence");
    assert_eq!(json["dock"], true);
    assert_eq!(json["widget_count"], 3);

    let empty = ServerMessage::presence(PresenceSnapshot::default());
    let json: Value = serde_json::to_value(&empty).unwrap();
    assert_eq!(json["dock"], false);
    assert_eq!(json["widget_count"], 0);
}

#[tokio::test]
async fn complete_via_message_marks_entry_completed_once() {
    let state = test_state().await;

    let entry_id = setup_spinning(&state).await;
    let reply = handle_message(&state, ClientMessage::Complete { entry_id }).await;
    assert!(matches!(reply, ServerMessage::CompleteOk { entry_id: id } if id == entry_id));
    let entry = state
        .queue_service
        .get_by_id(entry_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(entry.status, QueueStatus::Completed);

    let reply = handle_message(&state, ClientMessage::Complete { entry_id }).await;
    assert!(matches!(
        reply,
        ServerMessage::CompleteErr { entry_id: id, error }
        if id == entry_id && !error.is_empty()
    ));
}

#[tokio::test]
async fn complete_ok_and_err_serialize_with_type_tag() {
    let ok = ServerMessage::CompleteOk {
        entry_id: QueueEntryId::new(7),
    };
    let json: Value = serde_json::to_value(&ok).unwrap();
    assert_eq!(json["type"], "complete_ok");
    assert_eq!(json["entry_id"], 7);

    let err = ServerMessage::CompleteErr {
        entry_id: QueueEntryId::new(7),
        error: "nope".to_string(),
    };
    let json: Value = serde_json::to_value(&err).unwrap();
    assert_eq!(json["type"], "complete_err");
    assert_eq!(json["error"], "nope");
}
