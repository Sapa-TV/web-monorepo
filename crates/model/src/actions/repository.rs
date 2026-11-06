use std::future::Future;

use crate::actions::action::{Action, ActionId, ActionKind};
use crate::error::RepositoryError;

pub trait ActionRepository: Send + Sync {
    fn create(
        &self,
        name: &str,
        kind: ActionKind,
        enabled: bool,
    ) -> impl Future<Output = Result<Action, RepositoryError>> + Send;

    fn get_by_id(
        &self,
        id: ActionId,
    ) -> impl Future<Output = Result<Option<Action>, RepositoryError>> + Send;

    fn list(&self) -> impl Future<Output = Result<Vec<Action>, RepositoryError>> + Send;

    fn update(
        &self,
        action: Action,
    ) -> impl Future<Output = Result<Option<Action>, RepositoryError>> + Send;

    fn delete(&self, id: ActionId) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
}
