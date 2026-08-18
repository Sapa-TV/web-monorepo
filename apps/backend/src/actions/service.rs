use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::watch;

use crate::actions::action::{Action, ActionId, ActionKind};
use crate::actions::repository::ActionRepository;
use crate::error::ActionServiceError;

#[non_exhaustive]
pub struct ActionService<A>
where
    A: ActionRepository,
{
    repo: Arc<A>,
    revision: AtomicU64,
    lifecycle: watch::Sender<u64>,
}

impl<A> ActionService<A>
where
    A: ActionRepository,
{
    pub fn new(repo: Arc<A>) -> Self {
        Self {
            repo,
            revision: AtomicU64::new(0),
            lifecycle: watch::channel(0).0,
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
        kind: ActionKind,
        enabled: bool,
    ) -> Result<Action, ActionServiceError> {
        let action = self.repo.create(name, kind, enabled).await?;
        self.bump();
        Ok(action)
    }

    pub async fn get(&self, id: ActionId) -> Result<Option<Action>, ActionServiceError> {
        Ok(self.repo.get_by_id(id).await?)
    }

    pub async fn list(&self) -> Result<Vec<Action>, ActionServiceError> {
        Ok(self.repo.list().await?)
    }

    pub async fn update(&self, action: Action) -> Result<(), ActionServiceError> {
        if self.repo.update(action).await?.is_none() {
            return Err(ActionServiceError::ActionNotFound);
        }
        self.bump();
        Ok(())
    }

    pub async fn delete(&self, id: ActionId) -> Result<(), ActionServiceError> {
        if !self.repo.delete(id).await? {
            return Err(ActionServiceError::ActionNotFound);
        }
        self.bump();
        Ok(())
    }

    fn bump(&self) {
        let next = self.revision.fetch_add(1, Ordering::Relaxed) + 1;
        self.lifecycle.send_replace(next);
    }
}

#[cfg(test)]
mod tests {
    use crate::db::inmemory_actions::InMemoryActionRepository;

    use super::*;

    fn test_service() -> ActionService<InMemoryActionRepository> {
        ActionService::new(Arc::new(InMemoryActionRepository::new()))
    }

    #[tokio::test]
    async fn create_bumps_lifecycle() {
        let service = test_service();
        let mut rx = service.subscribe_lifecycle();
        service
            .create("reply", ActionKind::EnqueueRoulette, true)
            .await
            .unwrap();
        rx.changed().await.unwrap();
        assert_eq!(rx.borrow_and_update().clone(), 1);
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let service = test_service();
        assert!(service.get(ActionId::new(999)).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn update_missing_is_not_found() {
        let service = test_service();
        let err = service
            .update(Action {
                id: ActionId::new(999),
                name: "x".to_string(),
                kind: ActionKind::EnqueueRoulette,
                enabled: true,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, ActionServiceError::ActionNotFound));
    }

    #[tokio::test]
    async fn delete_missing_is_not_found() {
        let service = test_service();
        let err = service.delete(ActionId::new(999)).await.unwrap_err();
        assert!(matches!(err, ActionServiceError::ActionNotFound));
    }

    #[tokio::test]
    async fn delete_removes_and_bumps() {
        let service = test_service();
        let action = service
            .create("reply", ActionKind::EnqueueRoulette, true)
            .await
            .unwrap();
        service.delete(action.id).await.unwrap();
        assert!(service.get(action.id).await.unwrap().is_none());
    }
}
