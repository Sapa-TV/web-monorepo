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
            PlatformEventPayload::ChatMessage(msg) => {
                Self::chat(msg.user_id.clone(), msg.user_name.clone(), msg.text.clone())
            }
            PlatformEventPayload::RewardRedemption(red) => Self::reward(
                red.user_id.clone(),
                red.user_name.clone(),
                red.reward_title.clone(),
                red.reward_cost,
                red.user_input.clone(),
            ),
        }
    }
}

#[cfg(test)]
#[path = "event.test.rs"]
mod tests;
