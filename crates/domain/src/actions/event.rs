use std::sync::Arc;

use model::actions::action::{Action, ActionId, ActionKind, EventContext};
use model::event::PlatformEvent;

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
        let ctx = EventContext::from_payload(&source.payload);
        Self {
            source,
            action_id: action.id,
            kind: action.kind.clone(),
            ctx,
        }
    }
}

#[cfg(test)]
#[path = "event.test.rs"]
mod tests;
