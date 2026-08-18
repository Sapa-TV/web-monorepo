use std::future::Future;

use crate::actions::ActionId;
use crate::error::RepositoryError;
use crate::ingress::event::RuleTrigger;
use crate::rules::rule::{Rule, RuleConditions, RuleId};

pub trait RuleRepository: Send + Sync {
    fn create(
        &self,
        name: &str,
        enabled: bool,
        trigger: RuleTrigger,
        conditions: RuleConditions,
        action_id: ActionId,
    ) -> impl Future<Output = Result<Rule, RepositoryError>> + Send;

    fn get_by_id(
        &self,
        id: RuleId,
    ) -> impl Future<Output = Result<Option<Rule>, RepositoryError>> + Send;

    fn list(&self) -> impl Future<Output = Result<Vec<Rule>, RepositoryError>> + Send;

    fn update(
        &self,
        rule: Rule,
    ) -> impl Future<Output = Result<Option<Rule>, RepositoryError>> + Send;

    fn delete(&self, id: RuleId) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
}
