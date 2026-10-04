use std::sync::Arc;

use strum::IntoDiscriminant;
use tokio::sync::{broadcast, mpsc};

use crate::actions::ActionId;
use crate::actions::action::Action;
use crate::actions::event::ActionEvent;
use crate::actions::repository::ActionRepository;
use crate::actions::service::ActionService;
use crate::error::RuleServiceError;
use crate::ingress::event::{PlatformEvent, PlatformEventPayload};
use crate::rules::repository::RuleRepository;
use crate::rules::rule::{
    MessageConditions, MessageMatcher, RewardConditions, Rule, RuleConditions,
};
use crate::rules::service::RuleService;

#[non_exhaustive]
pub struct RuleEngine<R, A>
where
    R: RuleRepository,
    A: ActionRepository,
{
    rules: Arc<RuleService<R, A>>,
    actions: Arc<ActionService<A>>,
}

#[non_exhaustive]
struct ActiveRules {
    rules: Vec<Rule>,
    actions: Vec<(ActionId, Arc<Action>)>,
}

impl<R, A> RuleEngine<R, A>
where
    R: RuleRepository,
    A: ActionRepository,
{
    pub fn new(rules: Arc<RuleService<R, A>>, actions: Arc<ActionService<A>>) -> Self {
        Self { rules, actions }
    }

    pub async fn run(
        &self,
        mut rx: broadcast::Receiver<Arc<PlatformEvent>>,
        tx: mpsc::Sender<ActionEvent>,
    ) {
        let mut rule_lifecycle = self.rules.subscribe_lifecycle();
        let mut action_lifecycle = self.actions.subscribe_lifecycle();
        let mut active: Option<ActiveRules> = None;

        loop {
            if active.is_none() {
                match self.reload().await {
                    Ok(loaded) => active = Some(loaded),
                    Err(e) => tracing::warn!("rule engine reload failed: {e}"),
                }
            }

            tokio::select! {
                _ = rule_lifecycle.changed() => { active = None; }
                _ = action_lifecycle.changed() => { active = None; }
                event = rx.recv() => {
                    match event {
                        Ok(event) => {
                            if let Some(active) = &active {
                                self.process(&tx, &event, active).await;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            tracing::warn!("rule engine lagged, skipped {skipped} events");
                        }
                        Err(broadcast::error::RecvError::Closed) => return,
                    }
                }
            }
        }
    }

    async fn reload(&self) -> Result<ActiveRules, RuleServiceError> {
        let rules = self.rules.enabled_rules().await?;
        let mut actions = Vec::new();
        for rule in &rules {
            let Some(action) = self.actions.get(rule.action_id).await? else {
                continue;
            };
            if action.enabled {
                actions.push((rule.action_id, Arc::new(action)));
            }
        }
        Ok(ActiveRules { rules, actions })
    }

    async fn process(
        &self,
        tx: &mpsc::Sender<ActionEvent>,
        event: &Arc<PlatformEvent>,
        active: &ActiveRules,
    ) {
        let trigger = event.payload.discriminant();
        for rule in &active.rules {
            if rule.trigger != trigger || !conditions_match(&rule.conditions, &event.payload) {
                continue;
            }
            let Some((_, action)) = active.actions.iter().find(|(id, _)| *id == rule.action_id)
            else {
                continue;
            };
            let action_event = ActionEvent::from_action(Arc::clone(action), Arc::clone(event));
            if tx.send(action_event).await.is_err() {
                tracing::warn!("rule engine: action bus closed, stopping");
                return;
            }
        }
    }
}

pub fn conditions_match(conditions: &RuleConditions, payload: &PlatformEventPayload) -> bool {
    match (conditions, payload) {
        (
            RuleConditions::ChatMessage(MessageConditions {
                matcher, pattern, ..
            }),
            PlatformEventPayload::ChatMessage(msg),
        ) => {
            let Some(pattern) = pattern else {
                return false;
            };
            match matcher {
                MessageMatcher::Contains => msg.text.contains(pattern),
                MessageMatcher::StartsWith => msg.text.starts_with(pattern),
                MessageMatcher::Equals => msg.text == *pattern,
                MessageMatcher::EndsWith => msg.text.ends_with(pattern),
            }
        }
        (
            RuleConditions::RewardRedemption(RewardConditions { reward_id, .. }),
            PlatformEventPayload::RewardRedemption(red),
        ) => match reward_id {
            Some(id) => &red.reward_id == id,
            None => true,
        },
        _ => false,
    }
}

#[cfg(test)]
#[path = "engine.test.rs"]
mod tests;
