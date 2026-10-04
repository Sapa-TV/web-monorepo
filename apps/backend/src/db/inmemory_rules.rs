use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::nonpoison::Mutex;

use chrono::Utc;

use crate::actions::ActionId;
use crate::error::RepositoryError;
use crate::ingress::event::RuleTrigger;
use crate::rules::repository::RuleRepository;
use crate::rules::rule::{Rule, RuleConditions, RuleId};

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
        let rule = Rule::new(
            RuleId::new(id),
            name.to_string(),
            enabled,
            trigger,
            conditions,
            action_id,
            now,
            now,
        );
        self.rules.lock().push(rule.clone());
        Ok(rule)
    }

    async fn get_by_id(&self, id: RuleId) -> Result<Option<Rule>, RepositoryError> {
        Ok(self.rules.lock().iter().find(|r| r.id == id).cloned())
    }

    async fn list(&self) -> Result<Vec<Rule>, RepositoryError> {
        Ok(self.rules.lock().clone())
    }

    async fn update(&self, mut rule: Rule) -> Result<Option<Rule>, RepositoryError> {
        let mut rules = self.rules.lock();
        let Some(stored) = rules.iter_mut().find(|r| r.id == rule.id) else {
            return Ok(None);
        };
        rule.created_at = stored.created_at;
        rule.updated_at = Utc::now();
        *stored = rule;
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
#[path = "inmemory_rules.test.rs"]
mod tests;
