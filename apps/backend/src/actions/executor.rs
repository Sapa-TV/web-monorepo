use std::sync::Arc;

use tokio::sync::mpsc;
use twitch_oauth2::TwitchToken;

use crate::actions::action::{ActionKind, render};
use crate::actions::event::ActionEvent;
use crate::config::TwitchConfig;
use crate::error::ExecutorError;
use crate::ingress::twitch_auth::TwitchAuthService;
use crate::platform::{PlatformCredentialRepository, PlatformRepository};
use crate::queue::repository::QueueRepository;
use crate::queue::service::QueueService;
use crate::roulette::rarity::RarityRepository;
use crate::roulette::repository::RouletteSlotRepository;
use crate::user::UserId;
use crate::user::repository::UserRepository;
use crate::user::service::UserService;

pub struct ActionExecutor<Q, R, S, U, P, C>
where
    Q: QueueRepository,
    R: RarityRepository,
    S: RouletteSlotRepository,
    U: UserRepository,
    P: PlatformRepository,
    C: PlatformCredentialRepository,
{
    queue_service: Arc<QueueService<Q, R, S>>,
    user_service: Arc<UserService<U, P>>,
    twitch_auth: Option<Arc<TwitchAuthService<C>>>,
    broadcaster_id: String,
}

impl<Q, R, S, U, P, C> ActionExecutor<Q, R, S, U, P, C>
where
    Q: QueueRepository,
    R: RarityRepository,
    S: RouletteSlotRepository,
    U: UserRepository,
    P: PlatformRepository,
    C: PlatformCredentialRepository,
{
    pub fn new(
        queue_service: Arc<QueueService<Q, R, S>>,
        user_service: Arc<UserService<U, P>>,
        twitch_auth: Option<Arc<TwitchAuthService<C>>>,
        twitch_config: Option<Arc<TwitchConfig>>,
    ) -> Self {
        Self {
            queue_service,
            user_service,
            twitch_auth,
            broadcaster_id: twitch_config
                .map(|c| c.broadcaster_id.clone())
                .unwrap_or_default(),
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
                let message = render(message_template, &event.ctx);
                self.send_chat_message(&message).await?;
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

    async fn send_chat_message(&self, message: &str) -> Result<(), ExecutorError> {
        let Some(twitch_auth) = &self.twitch_auth else {
            return Err(ExecutorError::Chat("twitch is not configured".to_string()));
        };
        let token = twitch_auth
            .user_token()
            .await
            .map_err(|e| ExecutorError::Chat(e.to_string()))?;
        let sender_id = token
            .user_id()
            .ok_or_else(|| ExecutorError::Chat("token has no user_id".to_string()))?;
        let helix = twitch_auth.helix();
        helix
            .send_chat_message(&self.broadcaster_id, sender_id, message, &token)
            .await
            .map_err(|e| ExecutorError::Chat(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::mpsc;

    use crate::actions::action::{Action, ActionId};
    use crate::db::inmemory_config::InMemoryConfigRepository;
    use crate::db::inmemory_platform::InMemoryPlatformRepository;
    use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
    use crate::db::inmemory_queue::InMemoryQueueRepository;
    use crate::db::inmemory_rarity::InMemoryRarityRepository;
    use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
    use crate::db::inmemory_user::InMemoryUserRepository;
    use crate::ingress::event::PlatformEvent;
    use crate::ingress::twitch_auth::TwitchAuthService;
    use crate::platform::PlatformId;
    use crate::test_fixtures::test_state_with_data;

    use super::*;

    type TestExecutor = ActionExecutor<
        InMemoryQueueRepository,
        InMemoryRarityRepository,
        InMemoryRouletteSlotRepository,
        InMemoryUserRepository,
        InMemoryPlatformRepository,
        InMemoryPlatformCredentialRepository,
    >;

    async fn setup() -> TestExecutor {
        setup_with_twitch().await
    }

    async fn setup_with_twitch() -> TestExecutor {
        let queue_repo = Arc::new(InMemoryQueueRepository::new());
        let config_repo = Arc::new(InMemoryConfigRepository::new());
        let state = test_state_with_data(Arc::clone(&queue_repo), config_repo).await;
        let config = Arc::new(TwitchConfig {
            client_id: "cid".to_string(),
            client_secret: "cs".to_string(),
            broadcaster_id: "bc".to_string(),
            redirect_uri: "https://localhost/cb".to_string(),
            credentials_redirect_uri: "https://localhost/creds/cb".to_string(),
            csrf_ttl_secs: 600,
        });
        let twitch_auth = Arc::new(TwitchAuthService::new(
            Arc::clone(&config),
            Arc::clone(&state.credentials),
        ));
        ActionExecutor::new(
            Arc::clone(&state.queue_service),
            Arc::clone(&state.user_service),
            Some(twitch_auth),
            Some(config),
        )
    }

    async fn setup_without_twitch() -> TestExecutor {
        let queue_repo = Arc::new(InMemoryQueueRepository::new());
        let config_repo = Arc::new(InMemoryConfigRepository::new());
        let state = test_state_with_data(Arc::clone(&queue_repo), config_repo).await;
        ActionExecutor::new(
            Arc::clone(&state.queue_service),
            Arc::clone(&state.user_service),
            None,
            None,
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
            Arc::new(Action {
                id: ActionId::new(1),
                name: "test".to_string(),
                kind: action,
                enabled: true,
                created_at: now,
                updated_at: now,
            }),
            chat_event(event_id, user_id, user_name),
        )
    }

    #[tokio::test]
    async fn no_action_enqueues_nothing() {
        let executor = setup().await;
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
        let executor = setup().await;
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
        let executor = setup().await;
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
    async fn chat_reply_without_credentials_keeps_task_alive() {
        let executor = setup().await;
        let (tx, rx) = mpsc::channel(2);
        tx.send(action_event(
            ActionKind::ChatReply {
                message_template: "hi {username}".to_string(),
            },
            "msg-3",
            "1",
            "viewer",
        ))
        .await
        .unwrap();
        tx.send(action_event(
            ActionKind::EnqueueRoulette,
            "msg-4",
            "1",
            "viewer",
        ))
        .await
        .unwrap();
        drop(tx);
        executor.run(rx).await;

        let stats = executor.queue_service.count_by_status().await.unwrap();
        assert_eq!(stats.pending, 1, "task must survive a failing chat reply");
    }

    #[tokio::test]
    async fn enqueue_works_without_twitch_config() {
        let executor = setup_without_twitch().await;
        let (tx, rx) = mpsc::channel(2);
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

        let stats = executor.queue_service.count_by_status().await.unwrap();
        assert_eq!(stats.pending, 1);
    }

    #[tokio::test]
    async fn chat_reply_without_twitch_config_keeps_task_alive() {
        let executor = setup_without_twitch().await;
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
        assert_eq!(stats.pending, 1, "task must survive without twitch config");
    }
}
