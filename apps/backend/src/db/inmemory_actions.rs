use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::nonpoison::Mutex;

use chrono::Utc;

use crate::actions::action::{Action, ActionId, ActionKind};
use crate::actions::repository::ActionRepository;
use crate::error::RepositoryError;

#[non_exhaustive]
pub struct InMemoryActionRepository {
    actions: Mutex<Vec<Action>>,
    next_id: AtomicU32,
}

impl InMemoryActionRepository {
    pub fn new() -> Self {
        Self {
            actions: Mutex::new(Vec::new()),
            next_id: AtomicU32::new(1),
        }
    }
}

impl Default for InMemoryActionRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl ActionRepository for InMemoryActionRepository {
    async fn create(
        &self,
        name: &str,
        kind: ActionKind,
        enabled: bool,
    ) -> Result<Action, RepositoryError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Utc::now();
        let action = Action::new(ActionId::new(id), name.to_string(), kind, enabled, now, now);
        self.actions.lock().push(action.clone());
        Ok(action)
    }

    async fn get_by_id(&self, id: ActionId) -> Result<Option<Action>, RepositoryError> {
        Ok(self.actions.lock().iter().find(|a| a.id == id).cloned())
    }

    async fn list(&self) -> Result<Vec<Action>, RepositoryError> {
        Ok(self.actions.lock().clone())
    }

    async fn update(&self, action: Action) -> Result<Option<Action>, RepositoryError> {
        let mut actions = self.actions.lock();
        let Some(stored) = actions.iter_mut().find(|a| a.id == action.id) else {
            return Ok(None);
        };
        stored.name = action.name;
        stored.kind = action.kind;
        stored.enabled = action.enabled;
        stored.updated_at = Utc::now();
        Ok(Some(stored.clone()))
    }

    async fn delete(&self, id: ActionId) -> Result<bool, RepositoryError> {
        let mut actions = self.actions.lock();
        let len_before = actions.len();
        actions.retain(|a| a.id != id);
        Ok(actions.len() != len_before)
    }
}

#[cfg(test)]
#[path = "inmemory_actions.test.rs"]
mod tests;
