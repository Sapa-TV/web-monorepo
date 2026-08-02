use std::sync::Arc;
use std::sync::OnceLock;

use crate::config::Config;
use crate::db::inmemory_platform::InMemoryPlatformRepository;
use crate::db::inmemory_queue::InMemoryQueueRepository;
use crate::db::inmemory_rarity::InMemoryRarityRepository;
use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
use crate::db::inmemory_user::InMemoryUserRepository;
use crate::error::RepositoryError;
use crate::event::BroadcastEventPublisher;
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
use crate::user::UserId;
use crate::user::repository::UserRepository;

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
    pub user_repo: Arc<U>,
    pub platform_repo: Arc<P>,
    pub queue_service: QueueService<Q, R, S>,
    pub config: Config,
    pub event_publisher: BroadcastEventPublisher,
    pub guest_user_id: Arc<OnceLock<UserId>>,
    pub stream_status: Arc<StreamStatus>,
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
            user_repo: Arc::clone(&self.user_repo),
            platform_repo: Arc::clone(&self.platform_repo),
            queue_service: self.queue_service.clone(),
            config: self.config.clone(),
            event_publisher: self.event_publisher.clone(),
            guest_user_id: Arc::clone(&self.guest_user_id),
            stream_status: Arc::clone(&self.stream_status),
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

        let slot_service = Arc::new(RouletteSlotService::build(Arc::clone(&slot_repo)).await?);
        let rarity_service = Arc::new(RarityService::build(Arc::clone(&rarity_repo)).await?);
        let roulette = RouletteService::new(Arc::clone(&slot_service), self.random);
        let queue_service = QueueService::new(
            Arc::clone(&queue_repo),
            Arc::clone(&rarity_service),
            roulette,
            event_publisher.clone(),
            std::time::Duration::from_secs(self.config.roulette_timeout_secs),
        );

        Ok(AppState {
            slot_service,
            rarity_service,
            user_repo,
            platform_repo,
            queue_service,
            config: self.config,
            event_publisher,
            guest_user_id: Arc::new(OnceLock::new()),
            stream_status: Arc::new(StreamStatus::new()),
        })
    }
}
