use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::nonpoison::RwLock;

use tokio::sync::watch;

use crate::actions::ActionId;
use crate::actions::repository::ActionRepository;
use crate::actions::service::ActionService;
use crate::error::RuleServiceError;
use crate::ingress::event::RuleTrigger;
use crate::rules::repository::RuleRepository;
use crate::rules::rule::{MessageMatcher, Rule, RuleConditions, RuleId};

#[non_exhaustive]
pub struct RuleService<R, A>
where
    R: RuleRepository,
    A: ActionRepository,
{
    repo: Arc<R>,
    actions: Arc<ActionService<A>>,
    revision: AtomicU64,
    lifecycle: watch::Sender<u64>,
    enabled_cache: RwLock<Option<Vec<Rule>>>,
}

impl<R, A> RuleService<R, A>
where
    R: RuleRepository,
    A: ActionRepository,
{
    pub fn new(repo: Arc<R>, actions: Arc<ActionService<A>>) -> Self {
        Self {
            repo,
            actions,
            revision: AtomicU64::new(0),
            lifecycle: watch::channel(0).0,
            enabled_cache: RwLock::new(None),
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

    pub fn subscribe_lifecycle(&self) -> watch::Receiver<u64> {
        self.lifecycle.subscribe()
    }

    pub async fn create(
        &self,
        name: &str,
        enabled: bool,
        trigger: RuleTrigger,
        conditions: RuleConditions,
        action_id: ActionId,
    ) -> Result<Rule, RuleServiceError> {
        self.validate(&conditions, action_id).await?;
        let rule = self
            .repo
            .create(name, enabled, trigger, conditions, action_id)
            .await?;
        self.bump();
        Ok(rule)
    }

    pub async fn get(&self, id: RuleId) -> Result<Option<Rule>, RuleServiceError> {
        Ok(self.repo.get_by_id(id).await?)
    }

    pub async fn list(&self) -> Result<Vec<Rule>, RuleServiceError> {
        Ok(self.repo.list().await?)
    }

    pub async fn referenced_reward_ids(&self) -> Result<HashSet<String>, RuleServiceError> {
        Ok(self
            .list()
            .await?
            .into_iter()
            .filter_map(|rule| rule.referenced_reward_id().map(str::to_string))
            .collect())
    }

    pub async fn update(&self, rule: Rule) -> Result<(), RuleServiceError> {
        self.validate(&rule.conditions, rule.action_id).await?;
        if self.repo.update(rule).await?.is_none() {
            return Err(RuleServiceError::RuleNotFound);
        }
        self.bump();
        Ok(())
    }

    pub async fn delete(&self, id: RuleId) -> Result<(), RuleServiceError> {
        if !self.repo.delete(id).await? {
            return Err(RuleServiceError::RuleNotFound);
        }
        self.bump();
        Ok(())
    }

    pub async fn enabled_rules(&self) -> Result<Vec<Rule>, RuleServiceError> {
        if let Some(rules) = self.enabled_cache.read().clone() {
            return Ok(rules);
        }
        let rules = self
            .repo
            .list()
            .await?
            .into_iter()
            .filter(|r| r.enabled)
            .collect::<Vec<_>>();
        *self.enabled_cache.write() = Some(rules.clone());
        Ok(rules)
    }

    async fn validate(
        &self,
        conditions: &RuleConditions,
        action_id: ActionId,
    ) -> Result<(), RuleServiceError> {
        match conditions {
            RuleConditions::ChatMessage(message) => {
                if matches!(
                    message.matcher,
                    MessageMatcher::StartsWith | MessageMatcher::Equals | MessageMatcher::EndsWith
                ) && message.pattern.is_none()
                {
                    return Err(RuleServiceError::MissingPattern(
                        format!("{:?}", message.matcher).to_lowercase(),
                    ));
                }
            }
            RuleConditions::RewardRedemption(_) => {}
        }
        if self.actions.get(action_id).await?.is_none() {
            return Err(RuleServiceError::ActionNotFound);
        }
        Ok(())
    }

    fn bump(&self) {
        *self.enabled_cache.write() = None;
        let next = self.revision.fetch_add(1, Ordering::Relaxed) + 1;
        self.lifecycle.send_replace(next);
    }
}

#[cfg(test)]
mod tests {
    use crate::actions::action::ActionKind;
    use crate::db::inmemory_actions::InMemoryActionRepository;
    use crate::db::inmemory_rules::InMemoryRuleRepository;
    use crate::rules::rule::{MessageConditions, RewardConditions};

    use super::*;

    fn test_actions() -> Arc<ActionService<InMemoryActionRepository>> {
        Arc::new(ActionService::new(
            Arc::new(InMemoryActionRepository::new()),
        ))
    }

    fn test_service(
        actions: &Arc<ActionService<InMemoryActionRepository>>,
    ) -> RuleService<InMemoryRuleRepository, InMemoryActionRepository> {
        RuleService::new(Arc::new(InMemoryRuleRepository::new()), Arc::clone(actions))
    }

    fn chat_conditions(matcher: MessageMatcher) -> RuleConditions {
        RuleConditions::ChatMessage(MessageConditions {
            matcher,
            pattern: Some("!spin".to_string()),
        })
    }

    async fn seed_action(actions: &ActionService<InMemoryActionRepository>) -> ActionId {
        actions
            .create("enqueue", ActionKind::EnqueueRoulette, true)
            .await
            .unwrap()
            .id
    }

    #[tokio::test]
    async fn create_validates_action_exists() {
        let actions = test_actions();
        let service = test_service(&actions);
        let err = service
            .create(
                "rule",
                true,
                RuleTrigger::ChatMessage,
                chat_conditions(MessageMatcher::Contains),
                ActionId::new(999),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, RuleServiceError::ActionNotFound));
    }

    #[tokio::test]
    async fn create_requires_pattern_for_equals() {
        let actions = test_actions();
        let action_id = seed_action(&actions).await;
        let service = test_service(&actions);
        let err = service
            .create(
                "rule",
                true,
                RuleTrigger::ChatMessage,
                RuleConditions::ChatMessage(MessageConditions {
                    matcher: MessageMatcher::Equals,
                    pattern: None,
                }),
                action_id,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, RuleServiceError::MissingPattern(_)));
    }

    #[tokio::test]
    async fn create_bumps_and_seeds_cache() {
        let actions = test_actions();
        let action_id = seed_action(&actions).await;
        let service = test_service(&actions);
        let mut rx = service.subscribe_lifecycle();

        let rule = service
            .create(
                "rule",
                true,
                RuleTrigger::ChatMessage,
                chat_conditions(MessageMatcher::Contains),
                action_id,
            )
            .await
            .unwrap();
        rx.changed().await.unwrap();
        assert_eq!(rx.borrow_and_update().clone(), 1);

        let enabled = service.enabled_rules().await.unwrap();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, rule.id);
    }

    #[tokio::test]
    async fn enabled_cache_refreshes_after_update() {
        let actions = test_actions();
        let action_id = seed_action(&actions).await;
        let service = test_service(&actions);
        let rule = service
            .create(
                "rule",
                true,
                RuleTrigger::ChatMessage,
                chat_conditions(MessageMatcher::Contains),
                action_id,
            )
            .await
            .unwrap();
        assert_eq!(service.enabled_rules().await.unwrap().len(), 1);

        service
            .update(Rule {
                enabled: false,
                ..rule
            })
            .await
            .unwrap();
        assert!(service.enabled_rules().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn update_missing_is_not_found() {
        let actions = test_actions();
        let action_id = seed_action(&actions).await;
        let service = test_service(&actions);
        let err = service
            .update(Rule {
                id: RuleId::new(999),
                name: "x".to_string(),
                enabled: true,
                trigger: RuleTrigger::RewardRedemption,
                conditions: RuleConditions::RewardRedemption(RewardConditions { reward_id: None }),
                action_id,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, RuleServiceError::RuleNotFound));
    }

    #[tokio::test]
    async fn delete_missing_is_not_found() {
        let actions = test_actions();
        let service = test_service(&actions);
        let err = service.delete(RuleId::new(999)).await.unwrap_err();
        assert!(matches!(err, RuleServiceError::RuleNotFound));
    }
}
