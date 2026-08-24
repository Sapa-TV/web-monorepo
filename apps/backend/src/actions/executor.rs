use std::sync::Arc;

use crate::actions::action::{ActionKind, render};
use crate::actions::event::ActionEvent;
use crate::actions::platform::{ActionContext, PlatformActionExecutor};
use crate::actions::service::PlatformActionService;
use crate::error::ExecutorError;
use crate::platform::PlatformRepository;
use crate::queue::repository::QueueRepository;
use crate::queue::service::QueueService;
use crate::roulette::rarity::RarityRepository;
use crate::roulette::repository::RouletteSlotRepository;
use crate::user::UserId;
use crate::user::repository::UserRepository;
use crate::user::service::UserService;
use tokio::sync::mpsc;

pub struct ActionExecutor<Q, R, S, U, P, T>
where
    Q: QueueRepository,
    R: RarityRepository,
    S: RouletteSlotRepository,
    U: UserRepository,
    P: PlatformRepository,
    T: PlatformActionExecutor,
{
    queue_service: Arc<QueueService<Q, R, S>>,
    user_service: Arc<UserService<U, P>>,
    platform_actions: Arc<PlatformActionService<T>>,
}

impl<Q, R, S, U, P, T> ActionExecutor<Q, R, S, U, P, T>
where
    Q: QueueRepository,
    R: RarityRepository,
    S: RouletteSlotRepository,
    U: UserRepository,
    P: PlatformRepository,
    T: PlatformActionExecutor,
{
    pub fn new(
        queue_service: Arc<QueueService<Q, R, S>>,
        user_service: Arc<UserService<U, P>>,
        platform_actions: Arc<PlatformActionService<T>>,
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
                let ctx = ActionContext {
                    event_id: event.source.event_id.clone(),
                    user_id: event.ctx.user_id.clone(),
                    user_name: event.ctx.user_name.clone(),
                    channel_id: String::new(),
                };
                self.platform_actions
                    .send_chat_message(event.source.platform, &ctx, &text)
                    .await?;
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::nonpoison::Mutex;

    use tokio::sync::mpsc;

    use super::*;
    use crate::actions::action::{Action, ActionId};
    use crate::db::inmemory_platform::InMemoryPlatformRepository;
    use crate::db::inmemory_queue::InMemoryQueueRepository;
    use crate::db::inmemory_rarity::InMemoryRarityRepository;
    use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
    use crate::db::inmemory_user::InMemoryUserRepository;
    use crate::error::platform_action::ActionError as TestActionError;
    use crate::ingress::event::PlatformEvent;
    use crate::platform::PlatformId;
    use crate::test_fixtures::test_state_inmemory;

    struct SpyExecutor {
        inner: Arc<SpyInner>,
    }

    #[derive(Default)]
    struct SpyInner {
        fail: bool,
        texts: Mutex<Vec<String>>,
    }

    impl SpyExecutor {
        fn ok() -> Self {
            Self {
                inner: Arc::new(SpyInner::default()),
            }
        }

        fn failing() -> Self {
            Self {
                inner: Arc::new(SpyInner {
                    fail: true,
                    texts: Mutex::new(Vec::new()),
                }),
            }
        }

        fn texts(&self) -> Vec<String> {
            self.inner.texts.lock().clone()
        }
    }

    impl Clone for SpyExecutor {
        fn clone(&self) -> Self {
            Self {
                inner: Arc::clone(&self.inner),
            }
        }
    }

    impl PlatformActionExecutor for SpyExecutor {
        fn platform(&self) -> PlatformId {
            PlatformId::TWITCH
        }

        fn send_chat_message(
            &self,
            _ctx: &ActionContext,
            text: &str,
        ) -> impl Future<Output = Result<(), TestActionError>> + Send {
            let inner = Arc::clone(&self.inner);
            let text = text.to_string();
            async move {
                inner.texts.lock().push(text);
                if inner.fail {
                    Err(TestActionError::Api("spy failure".to_string()))
                } else {
                    Ok(())
                }
            }
        }
    }

    type TestExecutor = ActionExecutor<
        InMemoryQueueRepository,
        InMemoryRarityRepository,
        InMemoryRouletteSlotRepository,
        InMemoryUserRepository,
        InMemoryPlatformRepository,
        SpyExecutor,
    >;

    async fn setup(executor: SpyExecutor) -> TestExecutor {
        let (state, _queue_repo) = test_state_inmemory().await;
        let platform_actions = Arc::new(PlatformActionService::new().with_twitch(executor));
        ActionExecutor::new(
            Arc::clone(&state.queue_service),
            Arc::clone(&state.user_service),
            platform_actions,
        )
    }

    fn chat_event(event_id: &str, user_id: &str, user_name: &str) -> Arc<PlatformEvent> {
        Arc::new(PlatformEvent::chat_message(
            PlatformId::TWITCH,
            event_id,
            user_id.to_string(),
            user_name.to_string(),
            "hello".to_string(),
        ))
    }

    fn action_event(
        action: ActionKind,
        event_id: &str,
        user_id: &str,
        user_name: &str,
    ) -> ActionEvent {
        let now = chrono::Utc::now();
        ActionEvent::from_action(
            Arc::new(Action::new(
                ActionId::new(1),
                "test".to_string(),
                action,
                true,
                now,
                now,
            )),
            chat_event(event_id, user_id, user_name),
        )
    }

    #[tokio::test]
    async fn no_action_enqueues_nothing() {
        let executor = setup(SpyExecutor::ok()).await;
        let (tx, rx) = mpsc::channel(2);
        tx.send(action_event(ActionKind::NoAction, "msg-0", "1", "viewer"))
            .await
            .unwrap();
        drop(tx);
        executor.run(rx).await;
        let stats = executor.queue_service.count_by_status().await.unwrap();
        assert_eq!(stats.pending, 0);
    }

    #[tokio::test]
    async fn enqueue_roulette_creates_user_and_enqueues() {
        let executor = setup(SpyExecutor::ok()).await;
        let (tx, rx) = mpsc::channel(2);
        tx.send(action_event(
            ActionKind::EnqueueRoulette,
            "msg-1",
            "1",
            "viewer",
        ))
        .await
        .unwrap();
        drop(tx);
        executor.run(rx).await;

        let stats = executor.queue_service.count_by_status().await.unwrap();
        assert_eq!(stats.pending, 1);
        let user = executor
            .user_service
            .find_by_platform("twitch", "1")
            .await
            .unwrap()
            .expect("user created");
        assert_eq!(user.display_name, "viewer");
    }

    #[tokio::test]
    async fn enqueue_roulette_reuses_existing_user() {
        let executor = setup(SpyExecutor::ok()).await;
        let existing = executor.user_service.create("viewer").await.unwrap();
        executor
            .user_service
            .link_platform(existing.id, "twitch", "1", "viewer")
            .await
            .unwrap();

        let (tx, rx) = mpsc::channel(2);
        tx.send(action_event(
            ActionKind::EnqueueRoulette,
            "msg-2",
            "1",
            "viewer",
        ))
        .await
        .unwrap();
        drop(tx);
        executor.run(rx).await;

        let user = executor
            .user_service
            .find_by_platform("twitch", "1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(user.id, existing.id);
    }

    #[tokio::test]
    async fn chat_reply_is_rendered_and_dispatched_to_platform_executor() {
        let spy = SpyExecutor::ok();
        let executor = setup(spy.clone()).await;
        let (tx, rx) = mpsc::channel(2);
        tx.send(action_event(
            ActionKind::ChatReply {
                message_template: "привет, {username}!".to_string(),
            },
            "msg-3",
            "1",
            "viewer",
        ))
        .await
        .unwrap();
        drop(tx);
        executor.run(rx).await;

        assert_eq!(spy.texts(), vec!["привет, viewer!".to_string()]);
    }

    #[tokio::test]
    async fn chat_reply_platform_failure_keeps_task_alive() {
        let spy = SpyExecutor::failing();
        let executor = setup(spy.clone()).await;
        let (tx, rx) = mpsc::channel(2);
        tx.send(action_event(
            ActionKind::ChatReply {
                message_template: "hi {username}".to_string(),
            },
            "msg-4",
            "1",
            "viewer",
        ))
        .await
        .unwrap();
        tx.send(action_event(
            ActionKind::EnqueueRoulette,
            "msg-5",
            "1",
            "viewer",
        ))
        .await
        .unwrap();
        drop(tx);
        executor.run(rx).await;

        assert_eq!(
            spy.texts(),
            vec!["hi viewer".to_string()],
            "failing executor must still be called"
        );

        let stats = executor.queue_service.count_by_status().await.unwrap();
        assert_eq!(stats.pending, 1, "task must survive a failing chat reply");
    }

    #[tokio::test]
    async fn unregistered_platform_falls_back_to_unsupported_and_task_survives() {
        let (state, _queue_repo) = test_state_inmemory().await;
        let platform_actions: Arc<PlatformActionService<SpyExecutor>> =
            Arc::new(PlatformActionService::new());
        let executor = ActionExecutor::new(
            Arc::clone(&state.queue_service),
            Arc::clone(&state.user_service),
            platform_actions,
        );
        let (tx, rx) = mpsc::channel(2);
        tx.send(action_event(
            ActionKind::ChatReply {
                message_template: "hi {username}".to_string(),
            },
            "msg-6",
            "1",
            "viewer",
        ))
        .await
        .unwrap();
        tx.send(action_event(
            ActionKind::EnqueueRoulette,
            "msg-7",
            "1",
            "viewer",
        ))
        .await
        .unwrap();
        drop(tx);
        executor.run(rx).await;

        let stats = executor.queue_service.count_by_status().await.unwrap();
        assert_eq!(stats.pending, 1, "task must survive unsupported platform");
    }
}
