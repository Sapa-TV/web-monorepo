use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::watch;

use crate::actions::action::{Action, ActionId, ActionKind};
use crate::actions::platform::{ActionContext, PlatformActionExecutor};
use crate::actions::repository::ActionRepository;
use crate::error::ActionServiceError;
use crate::error::platform_action::ActionError;
use crate::platform::PlatformId;

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

pub struct PlatformActionService<T>
where
    T: PlatformActionExecutor,
{
    twitch: Option<T>,
}

impl<T> Default for PlatformActionService<T>
where
    T: PlatformActionExecutor,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> PlatformActionService<T>
where
    T: PlatformActionExecutor,
{
    pub fn new() -> Self {
        Self { twitch: None }
    }

    pub fn with_twitch(mut self, executor: T) -> Self {
        self.twitch = Some(executor);
        self
    }

    pub async fn send_chat_message(
        &self,
        platform: PlatformId,
        ctx: &ActionContext,
        text: &str,
    ) -> Result<(), ActionError> {
        match platform {
            PlatformId::TWITCH => match &self.twitch {
                Some(executor) => executor.send_chat_message(ctx, text).await,
                None => unsupported(platform, "send_chat_message"),
            },
            other => unsupported(other, "send_chat_message"),
        }
    }
}

fn unsupported(platform: PlatformId, action: &str) -> Result<(), ActionError> {
    tracing::warn!(
        platform = platform.name(),
        action,
        "capability not supported"
    );
    Err(ActionError::Unsupported)
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
            .update(Action::new(
                ActionId::new(999),
                "x".to_string(),
                ActionKind::EnqueueRoulette,
                true,
                chrono::Utc::now(),
                chrono::Utc::now(),
            ))
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

#[cfg(test)]
mod platform_action_tests {
    use std::sync::Arc;
    use std::sync::nonpoison::Mutex;

    use super::*;

    struct SpyExecutor {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl Clone for SpyExecutor {
        fn clone(&self) -> Self {
            Self {
                calls: Arc::clone(&self.calls),
            }
        }
    }

    impl SpyExecutor {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn texts(&self) -> Vec<String> {
            self.calls.lock().clone()
        }
    }

    impl PlatformActionExecutor for SpyExecutor {
        fn platform(&self) -> PlatformId {
            PlatformId::TWITCH
        }

        async fn send_chat_message(
            &self,
            _ctx: &ActionContext,
            text: &str,
        ) -> Result<(), ActionError> {
            self.calls.lock().push(text.to_string());
            Ok(())
        }
    }

    fn action_ctx() -> ActionContext {
        ActionContext::new("e-1".to_string(), "42".to_string(), "viewer".to_string())
    }

    #[tokio::test]
    async fn empty_service_reports_unsupported() {
        let service = PlatformActionService::<SpyExecutor>::new();
        let err = service
            .send_chat_message(PlatformId::TWITCH, &action_ctx(), "hi")
            .await
            .unwrap_err();
        assert!(matches!(err, ActionError::Unsupported));
    }

    #[tokio::test]
    async fn non_twitch_platform_is_unsupported_even_with_executor() {
        let service = PlatformActionService::new().with_twitch(SpyExecutor::new());
        let err = service
            .send_chat_message(PlatformId::YOUTUBE, &action_ctx(), "hi")
            .await
            .unwrap_err();
        assert!(matches!(err, ActionError::Unsupported));
    }

    #[tokio::test]
    async fn registered_executor_receives_text() {
        let spy = SpyExecutor::new();
        let service = PlatformActionService::new().with_twitch(spy.clone());

        service
            .send_chat_message(PlatformId::TWITCH, &action_ctx(), "привет!")
            .await
            .unwrap();

        assert_eq!(spy.texts(), vec!["привет!".to_string()]);
    }
}
