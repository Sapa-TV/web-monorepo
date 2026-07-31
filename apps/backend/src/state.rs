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
use crate::queue::service::QueueService;
use crate::random::StandartRandomProvider;
use crate::roulette::machine::RouletteService;
use crate::roulette::slot_service::RouletteSlotService;
use crate::user::UserId;

#[derive(Clone)]
#[non_exhaustive]
pub struct AppState {
    pub slot_service: Arc<RouletteSlotService<Arc<InMemoryRouletteSlotRepository>>>,
    pub rarity_repo: Arc<InMemoryRarityRepository>,
    pub user_repo: Arc<InMemoryUserRepository>,
    pub platform_repo: Arc<InMemoryPlatformRepository>,
    pub queue_repo: Arc<InMemoryQueueRepository>,
    pub queue_service: QueueService,
    pub config: Config,
    pub event_publisher: BroadcastEventPublisher,
    pub guest_user_id: Arc<OnceLock<UserId>>,
}

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
        let roulette = RouletteService::new(Arc::clone(&slot_service), self.random);
        let queue_service = QueueService::new(
            Arc::clone(&queue_repo),
            Arc::clone(&rarity_repo),
            roulette,
            event_publisher.clone(),
            std::time::Duration::from_secs(self.config.roulette_timeout_secs),
        );

        Ok(AppState {
            slot_service,
            rarity_repo,
            user_repo,
            platform_repo,
            queue_repo,
            queue_service,
            config: self.config,
            event_publisher,
            guest_user_id: Arc::new(OnceLock::new()),
        })
    }
}
