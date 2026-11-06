use std::sync::Arc;

use crate::actions::event::ActionEvent;
use crate::actions::platform::{ActionContext, PlatformActionExecutor};
use crate::actions::service::PlatformActionService;
use crate::error::ExecutorError;
use crate::queue::service::QueueService;
use crate::user::service::UserService;
use model::actions::action::{ActionKind, render};
use model::platform::PlatformRepository;
use model::queue::repository::QueueRepository;
use model::roulette::rarity::RarityRepository;
use model::roulette::repository::RouletteSlotRepository;
use model::user::UserId;
use model::user::repository::UserRepository;
use tokio::sync::mpsc;

#[non_exhaustive]
pub struct ActionExecutor<Q, R, S, U, P, T, V = T>
where
    Q: QueueRepository,
    R: RarityRepository,
    S: RouletteSlotRepository,
    U: UserRepository,
    P: PlatformRepository,
    T: PlatformActionExecutor,
    V: PlatformActionExecutor,
{
    pub queue_service: Arc<QueueService<Q, R, S>>,
    pub user_service: Arc<UserService<U, P>>,
    pub platform_actions: Arc<PlatformActionService<T, V>>,
}

impl<Q, R, S, U, P, T, V> ActionExecutor<Q, R, S, U, P, T, V>
where
    Q: QueueRepository,
    R: RarityRepository,
    S: RouletteSlotRepository,
    U: UserRepository,
    P: PlatformRepository,
    T: PlatformActionExecutor,
    V: PlatformActionExecutor,
{
    pub fn new(
        queue_service: Arc<QueueService<Q, R, S>>,
        user_service: Arc<UserService<U, P>>,
        platform_actions: Arc<PlatformActionService<T, V>>,
    ) -> Self {
        Self {
            queue_service,
            user_service,
            platform_actions,
        }
    }

    pub async fn run(&self, mut rx: mpsc::Receiver<ActionEvent>) {
        while let Some(event) = rx.recv().await {
            if let Err(e) = self.execute(&event).await {
                tracing::warn!(action_id = %event.action_id, error = %e, "action execution failed");
            }
        }
    }

    async fn execute(&self, event: &ActionEvent) -> Result<(), ExecutorError> {
        match &event.kind {
            ActionKind::NoAction => {}
            ActionKind::EnqueueRoulette => {
                let user_id = self.ensure_user(event).await?;
                self.queue_service
                    .enqueue(user_id, &event.ctx.user_name)
                    .await?;
            }
            ActionKind::ChatReply { message_template } => {
                let text = render(message_template, &event.ctx);
                let ctx = ActionContext::new(
                    event.source.event_id.clone(),
                    event.ctx.user_id.clone(),
                    event.ctx.user_name.clone(),
                );
                self.platform_actions
                    .send_chat_message(event.source.platform, &ctx, &text)
                    .await?;
            }
            _ => {
                tracing::warn!(kind = ?event.kind, "unsupported action kind");
            }
        }
        Ok(())
    }

    async fn ensure_user(&self, event: &ActionEvent) -> Result<UserId, ExecutorError> {
        Ok(self
            .user_service
            .ensure_user_by_platform(
                event.source.platform.name(),
                &event.ctx.user_id,
                &event.ctx.user_name,
            )
            .await?)
    }
}
