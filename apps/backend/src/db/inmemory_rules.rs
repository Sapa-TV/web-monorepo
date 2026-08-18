use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::nonpoison::Mutex;

use chrono::Utc;

use crate::actions::ActionId;
use crate::error::RepositoryError;
use crate::ingress::event::RuleTrigger;
use crate::rules::rule::{Rule, RuleConditions, RuleId};
use crate::rules::repository::RuleRepository;

#[non_exhaustive]
pub struct InMemoryRuleRepository {
    rules: Mutex<Vec<Rule>>,
    next_id: AtomicU32,
}

impl InMemoryRuleRepository {
    pub fn new() -> Self {
        Self {
            rules: Mutex::new(Vec::new()),
            next_id: AtomicU32::new(1),
        }
    }
}

impl Default for InMemoryRuleRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleRepository for InMemoryRuleRepository {
    async fn create(
        &self,
        name: &str,
        enabled: bool,
        trigger: RuleTrigger,
        conditions: RuleConditions,
        action_id: ActionId,
    ) -> Result<Rule, RepositoryError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Utc::now();
        let rule = Rule {
            id: RuleId::new(id),
            name: name.to_string(),
            enabled,
            trigger,
            conditions,
            action_id,
            created_at: now,
            updated_at: now,
        };
        self.rules.lock().push(rule.clone());
        Ok(rule)
    }

    async fn get_by_id(&self, id: RuleId) -> Result<Option<Rule>, RepositoryError> {
        Ok(self.rules.lock().iter().find(|r| r.id == id).cloned())
    }

    async fn list(&self) -> Result<Vec<Rule>, RepositoryError> {
        Ok(self.rules.lock().clone())
    }

    async fn update(&self, rule: Rule) -> Result<Option<Rule>, RepositoryError> {
        let mut rules = self.rules.lock();
        let Some(stored) = rules.iter_mut().find(|r| r.id == rule.id) else {
            return Ok(None);
        };
        *stored = Rule {
            created_at: stored.created_at,
            updated_at: Utc::now(),
            ..rule
        };
        Ok(Some(stored.clone()))
    }

    async fn delete(&self, id: RuleId) -> Result<bool, RepositoryError> {
        let mut rules = self.rules.lock();
        let len_before = rules.len();
        rules.retain(|r| r.id != id);
        Ok(rules.len() != len_before)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::rule::{MessageConditions, MessageMatcher, RewardConditions};

    fn conditions(trigger: RuleTrigger) -> RuleConditions {
        match trigger {
            RuleTrigger::ChatMessage => RuleConditions::ChatMessage(MessageConditions {
                matcher: MessageMatcher::Contains,
                pattern: Some("!spin".to_string()),
            }),
            RuleTrigger::RewardRedemption => {
                RuleConditions::RewardRedemption(RewardConditions { reward_id: None })
            }
        }
    }

    #[tokio::test]
    async fn create_and_get() {
        let repo = InMemoryRuleRepository::new();
        let rule = repo
            .create(
                "chat-spin",
                true,
                RuleTrigger::ChatMessage,
                conditions(RuleTrigger::ChatMessage),
                ActionId::new(1),
            )
            .await
            .unwrap();
        assert_eq!(rule.id, RuleId::new(1));

        let fetched = repo.get_by_id(RuleId::new(1)).await.unwrap().unwrap();
        assert_eq!(fetched.name, "chat-spin");
        assert_eq!(fetched.action_id, ActionId::new(1));
    }

    #[tokio::test]
    async fn list_returns_all() {
        let repo = InMemoryRuleRepository::new();
        repo.create(
            "a",
            true,
            RuleTrigger::ChatMessage,
            conditions(RuleTrigger::ChatMessage),
            ActionId::new(1),
        )
        .await
        .unwrap();
        repo.create(
            "b",
            false,
            RuleTrigger::RewardRedemption,
            conditions(RuleTrigger::RewardRedemption),
            ActionId::new(2),
        )
        .await
        .unwrap();
        assert_eq!(repo.list().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn update_replaces_fields_and_touches_updated_at() {
        let repo = InMemoryRuleRepository::new();
        repo.create(
            "a",
            true,
            RuleTrigger::ChatMessage,
            conditions(RuleTrigger::ChatMessage),
            ActionId::new(1),
        )
        .await
        .unwrap();
        let original = repo.get_by_id(RuleId::new(1)).await.unwrap().unwrap();
        let updated = repo
            .update(Rule {
                name: "renamed".to_string(),
                enabled: false,
                ..original.clone()
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.name, "renamed");
        assert!(!updated.enabled);
        assert_eq!(updated.created_at, original.created_at);
        assert!(updated.updated_at > original.updated_at);

        assert!(
            repo.update(Rule {
                id: RuleId::new(99),
                ..original
            })
            .await
            .unwrap()
            .is_none()
        );
    }

    #[tokio::test]
    async fn delete_removes_entry() {
        let repo = InMemoryRuleRepository::new();
        repo.create(
            "a",
            true,
            RuleTrigger::ChatMessage,
            conditions(RuleTrigger::ChatMessage),
            ActionId::new(1),
        )
        .await
        .unwrap();
        assert!(repo.delete(RuleId::new(1)).await.unwrap());
        assert!(!repo.delete(RuleId::new(1)).await.unwrap());
        assert!(repo.get_by_id(RuleId::new(1)).await.unwrap().is_none());
    }
}
