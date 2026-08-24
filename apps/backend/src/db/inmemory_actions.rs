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
mod tests {
    use super::*;

    fn reply_kind() -> ActionKind {
        ActionKind::ChatReply {
            message_template: "hi {username}".to_string(),
        }
    }

    #[tokio::test]
    async fn create_and_get() {
        let repo = InMemoryActionRepository::new();
        let action = repo.create("reply", reply_kind(), true).await.unwrap();
        assert_eq!(action.id, ActionId::new(1));

        let fetched = repo.get_by_id(ActionId::new(1)).await.unwrap().unwrap();
        assert_eq!(fetched.name, "reply");
        assert_eq!(fetched.kind, reply_kind());
    }

    #[tokio::test]
    async fn list_returns_all() {
        let repo = InMemoryActionRepository::new();
        repo.create("a", reply_kind(), true).await.unwrap();
        repo.create("b", ActionKind::EnqueueRoulette, false)
            .await
            .unwrap();
        assert_eq!(repo.list().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn update_replaces_fields_and_touches_updated_at() {
        let repo = InMemoryActionRepository::new();
        repo.create("a", reply_kind(), true).await.unwrap();
        let original = repo.get_by_id(ActionId::new(1)).await.unwrap().unwrap();
        let updated = repo
            .update(Action::new(
                original.id,
                "renamed".to_string(),
                ActionKind::EnqueueRoulette,
                original.enabled,
                original.created_at,
                original.updated_at,
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.name, "renamed");
        assert_eq!(updated.kind, ActionKind::EnqueueRoulette);
        assert_eq!(updated.created_at, original.created_at);
        assert!(updated.updated_at > original.updated_at);

        let ghost = Action::new(
            ActionId::new(99),
            original.name.clone(),
            original.kind.clone(),
            original.enabled,
            original.created_at,
            original.updated_at,
        );
        assert!(repo.update(ghost).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_removes_entry() {
        let repo = InMemoryActionRepository::new();
        repo.create("a", reply_kind(), true).await.unwrap();
        assert!(repo.delete(ActionId::new(1)).await.unwrap());
        assert!(!repo.delete(ActionId::new(1)).await.unwrap());
        assert!(repo.get_by_id(ActionId::new(1)).await.unwrap().is_none());
    }
}
