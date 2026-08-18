use std::sync::Arc;

use crate::actions::action::{Action, ActionId, ActionKind, EventContext};
use crate::ingress::event::{PlatformEvent, PlatformEventPayload};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ActionEvent {
    pub source: Arc<PlatformEvent>,
    pub action_id: ActionId,
    pub kind: ActionKind,
    pub ctx: EventContext,
}

impl ActionEvent {
    pub fn from_action(action: Arc<Action>, source: Arc<PlatformEvent>) -> Self {
        let ctx = EventContext::from(&source.payload);
        Self {
            source,
            action_id: action.id,
            kind: action.kind.clone(),
            ctx,
        }
    }
}

impl From<&PlatformEventPayload> for EventContext {
    fn from(payload: &PlatformEventPayload) -> Self {
        match payload {
            PlatformEventPayload::ChatMessage(msg) => Self {
                user_id: msg.user_id.clone(),
                user_name: msg.user_name.clone(),
                text: msg.text.clone(),
                ..Self::default()
            },
            PlatformEventPayload::RewardRedemption(red) => Self {
                user_id: red.user_id.clone(),
                user_name: red.user_name.clone(),
                reward_title: red.reward_title.clone(),
                reward_cost: red.reward_cost,
                user_input: red.user_input.clone(),
                ..Self::default()
            },
        }
    }
}

#[cfg(test)]
mod tests {
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
        Arc::new(Action {
            id: ActionId::new(1),
            name: "test".to_string(),
            kind,
            enabled: true,
            created_at: now,
            updated_at: now,
        })
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
}
