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

pub struct PlatformActionService<T, V = T>
where
    T: PlatformActionExecutor,
    V: PlatformActionExecutor,
{
    twitch: Option<T>,
    vk_video_live: Option<V>,
}

impl<T> Default for PlatformActionService<T>
where
    T: PlatformActionExecutor,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, V> PlatformActionService<T, V>
where
    T: PlatformActionExecutor,
    V: PlatformActionExecutor,
{
    pub fn new() -> Self {
        Self {
            twitch: None,
            vk_video_live: None,
        }
    }

    pub fn with_twitch(mut self, executor: T) -> Self {
        self.twitch = Some(executor);
        self
    }

    pub fn with_vk_video_live(mut self, executor: V) -> Self {
        self.vk_video_live = Some(executor);
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
            PlatformId::VK_VIDEO_LIVE => match &self.vk_video_live {
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
#[path = "service.test.rs"]
mod tests;

#[cfg(test)]
#[path = "service.platform_action.test.rs"]
mod platform_action_tests;
