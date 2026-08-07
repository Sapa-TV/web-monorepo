use std::sync::Arc;
use std::time::Duration;

use crate::admin::auth::AdminAuthService;
use crate::config::Config;
use crate::db::inmemory_platform::InMemoryPlatformRepository;
use crate::db::inmemory_queue::InMemoryQueueRepository;
use crate::db::inmemory_rarity::InMemoryRarityRepository;
use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
use crate::db::inmemory_user::InMemoryUserRepository;
use crate::error::RepositoryError;
use crate::event::BroadcastEventPublisher;
use crate::ingress::{EventIngress, spawn_logging_handler};
use crate::platform::PlatformRepository;
use crate::queue::repository::QueueRepository;
use crate::queue::service::QueueService;
use crate::random::StandartRandomProvider;
use crate::roulette::machine::RouletteService;
use crate::roulette::rarity::RarityRepository;
use crate::roulette::rarity_service::RarityService;
use crate::roulette::repository::RouletteSlotRepository;
use crate::roulette::slot_service::RouletteSlotService;
use crate::stream::StreamStatus;
use crate::user::repository::UserRepository;
use crate::user::service::UserService;

#[non_exhaustive]
pub struct UniAppState<Q, R, U, P, S>
where
    Q: QueueRepository,
    R: RarityRepository,
    U: UserRepository,
    P: PlatformRepository,
    S: RouletteSlotRepository,
{
    pub slot_service: Arc<RouletteSlotService<Arc<S>>>,
    pub rarity_service: Arc<RarityService<Arc<R>>>,
    pub user_service: Arc<UserService<U, P>>,
    pub queue_service: QueueService<Q, R, S>,
    pub config: Config,
    pub event_publisher: BroadcastEventPublisher,
    pub stream_status: Arc<StreamStatus>,
    pub ingress: Arc<EventIngress>,
    pub admin_auth: Arc<AdminAuthService>,
}

impl<Q, R, U, P, S> Clone for UniAppState<Q, R, U, P, S>
where
    Q: QueueRepository,
    R: RarityRepository,
    U: UserRepository,
    P: PlatformRepository,
    S: RouletteSlotRepository,
{
    fn clone(&self) -> Self {
        Self {
            slot_service: Arc::clone(&self.slot_service),
            rarity_service: Arc::clone(&self.rarity_service),
            user_service: Arc::clone(&self.user_service),
            queue_service: self.queue_service.clone(),
            config: self.config.clone(),
            event_publisher: self.event_publisher.clone(),
            stream_status: Arc::clone(&self.stream_status),
            ingress: Arc::clone(&self.ingress),
            admin_auth: Arc::clone(&self.admin_auth),
        }
    }
}

pub type AppState = UniAppState<
    InMemoryQueueRepository,
    InMemoryRarityRepository,
    InMemoryUserRepository,
    InMemoryPlatformRepository,
    InMemoryRouletteSlotRepository,
>;

pub struct AppStateBuilder {
    random: StandartRandomProvider,
    config: Config,
    seeded: bool,
}

impl AppStateBuilder {
    pub fn new(random: StandartRandomProvider, config: &Config) -> Self {
        Self {
            random,
            config: config.clone(),
            seeded: true,
        }
    }

    #[cfg(test)]
    pub fn with_empty_repos(mut self) -> Self {
        self.seeded = false;
        self
    }

    pub async fn build(self) -> Result<AppState, RepositoryError> {
        let slot_repo = Arc::new(if self.seeded {
            InMemoryRouletteSlotRepository::new_seeded()
        } else {
            InMemoryRouletteSlotRepository::seed(vec![])
        });
        let rarity_repo = Arc::new(if self.seeded {
            InMemoryRarityRepository::new_seeded()
        } else {
            InMemoryRarityRepository::seed(vec![])
        });
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let platform_repo = Arc::new(InMemoryPlatformRepository::new_seeded());
        let queue_repo = Arc::new(InMemoryQueueRepository::new());
        let event_publisher = BroadcastEventPublisher::new();
        let ingress = Arc::new(EventIngress::new());
        spawn_logging_handler(ingress.subscribe());

        let slot_service = Arc::new(RouletteSlotService::build(Arc::clone(&slot_repo)).await?);
        let rarity_service = Arc::new(RarityService::build(Arc::clone(&rarity_repo)).await?);
        let roulette = RouletteService::new(Arc::clone(&slot_service), self.random);
        let queue_service = QueueService::new(
            Arc::clone(&queue_repo),
            Arc::clone(&rarity_service),
            roulette,
            event_publisher.clone(),
            Duration::from_secs(self.config.roulette_timeout_secs),
            Duration::from_secs(self.config.retention_secs),
        );
        let user_service = Arc::new(UserService::new(user_repo, platform_repo));
        let admin_auth = Arc::new(AdminAuthService::new(self.config.twitch.clone()));

        Ok(AppState {
            slot_service,
            rarity_service,
            user_service,
            queue_service,
            config: self.config,
            event_publisher,
            stream_status: Arc::new(StreamStatus::new()),
            ingress,
            admin_auth,
        })
    }
}
