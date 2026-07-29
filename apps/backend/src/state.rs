use std::sync::Arc;

use crate::config::Config;
use crate::db::inmemory_platform::InMemoryPlatformRepository;
use crate::db::inmemory_queue::InMemoryQueueRepository;
use crate::db::inmemory_rarity::InMemoryRarityRepository;
use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
use crate::db::inmemory_user::InMemoryUserRepository;
use crate::event::BroadcastEventPublisher;
use crate::queue::service::QueueService;
use crate::random::StandartRandomProvider;

#[derive(Clone)]
#[non_exhaustive]
pub struct AppState {
    pub slot_repo: Arc<InMemoryRouletteSlotRepository>,
    pub rarity_repo: Arc<InMemoryRarityRepository>,
    pub user_repo: Arc<InMemoryUserRepository>,
    pub platform_repo: Arc<InMemoryPlatformRepository>,
    pub queue_repo: Arc<InMemoryQueueRepository>,
    pub queue_service: QueueService,
    pub random: StandartRandomProvider,
    pub event_publisher: BroadcastEventPublisher,
}

impl AppState {
    pub fn new(random: StandartRandomProvider, config: &Config) -> Self {
        let slot_repo = Arc::new(InMemoryRouletteSlotRepository::new_seeded());
        let rarity_repo = Arc::new(InMemoryRarityRepository::new_seeded());
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let platform_repo = Arc::new(InMemoryPlatformRepository::new_seeded());
        let queue_repo = Arc::new(InMemoryQueueRepository::new());
        let event_publisher = BroadcastEventPublisher::new();
        let queue_service = QueueService::new(
            Arc::clone(&queue_repo),
            Arc::clone(&slot_repo),
            Arc::clone(&rarity_repo),
            Arc::clone(&user_repo),
            random.clone(),
            event_publisher.clone(),
            std::time::Duration::from_secs(config.roulette_timeout_secs),
        );
        Self {
            slot_repo,
            rarity_repo,
            user_repo,
            platform_repo,
            queue_repo,
            queue_service,
            random,
            event_publisher,
        }
    }
}

#[cfg(test)]
impl AppState {
    pub fn new_test_state(random: StandartRandomProvider) -> Self {
        let slot_repo = Arc::new(InMemoryRouletteSlotRepository::new());
        let rarity_repo = Arc::new(InMemoryRarityRepository::new());
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let platform_repo = Arc::new(InMemoryPlatformRepository::new_seeded());
        let queue_repo = Arc::new(InMemoryQueueRepository::new());
        let event_publisher = BroadcastEventPublisher::new();
        let queue_service = QueueService::new(
            Arc::clone(&queue_repo),
            Arc::clone(&slot_repo),
            Arc::clone(&rarity_repo),
            Arc::clone(&user_repo),
            random.clone(),
            event_publisher.clone(),
            std::time::Duration::from_secs(10),
        );
        Self {
            slot_repo,
            rarity_repo,
            user_repo,
            platform_repo,
            queue_repo,
            queue_service,
            random,
            event_publisher,
        }
    }
}
