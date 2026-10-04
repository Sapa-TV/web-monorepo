use std::sync::Arc;

use tokio::sync::mpsc;

use crate::actions::action::ActionKind;
use crate::ingress::event::PlatformEvent;
use crate::platform::PlatformId;

use super::*;

fn chat_event() -> Arc<PlatformEvent> {
    Arc::new(PlatformEvent::chat_message(
        PlatformId::TWITCH,
        "msg-1",
        "1".to_string(),
        "viewer".to_string(),
        "hello".to_string(),
    ))
}

fn reward_event() -> Arc<PlatformEvent> {
    Arc::new(PlatformEvent::reward_redemption(
        PlatformId::TWITCH,
        "red-1",
        "1".to_string(),
        "viewer".to_string(),
        "reward-9".to_string(),
        "Spin".to_string(),
        500,
        "please".to_string(),
        "unfulfilled".to_string(),
    ))
}

fn action(kind: ActionKind) -> Arc<Action> {
    let now = chrono::Utc::now();
    Arc::new(Action::new(
        ActionId::new(1),
        "test".to_string(),
        kind,
        true,
        now,
        now,
    ))
}

#[test]
fn ctx_from_chat_payload() {
    let event = chat_event();
    let ctx = EventContext::from(&event.payload);
    assert_eq!(ctx.user_id, "1");
    assert_eq!(ctx.user_name, "viewer");
    assert_eq!(ctx.text, "hello");
    assert_eq!(ctx.reward_cost, 0);
}

#[test]
fn ctx_from_reward_payload() {
    let event = reward_event();
    let ctx = EventContext::from(&event.payload);
    assert_eq!(ctx.user_id, "1");
    assert_eq!(ctx.user_name, "viewer");
    assert_eq!(ctx.reward_title, "Spin");
    assert_eq!(ctx.reward_cost, 500);
    assert_eq!(ctx.user_input, "please");
    assert_eq!(ctx.text, "");
}

#[test]
fn action_event_keeps_source_and_kind() {
    let event = chat_event();
    let action = action(ActionKind::EnqueueRoulette);
    let action_event = ActionEvent::from_action(Arc::clone(&action), Arc::clone(&event));
    assert_eq!(action_event.action_id, action.id);
    assert_eq!(action_event.kind, ActionKind::EnqueueRoulette);
    assert_eq!(action_event.ctx.user_name, "viewer");
    assert!(Arc::ptr_eq(&action_event.source, &event));
}

#[tokio::test]
async fn action_event_roundtrips_through_channel() {
    let (tx, mut rx) = mpsc::channel(4);
    let event = reward_event();
    let action = action(ActionKind::ChatReply {
        message_template: "hi {username}".to_string(),
    });
    let action_event = ActionEvent::from_action(Arc::clone(&action), Arc::clone(&event));
    tx.send(action_event.clone()).await.unwrap();

    let received = rx.recv().await.unwrap();
    assert_eq!(received.action_id, action_event.action_id);
    assert_eq!(received.kind, action_event.kind);
    assert_eq!(received.ctx.reward_cost, 500);
}
