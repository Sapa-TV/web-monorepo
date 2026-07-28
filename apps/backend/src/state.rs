use std::sync::Arc;

use crate::db::inmemory_platform::InMemoryPlatformRepository;
use crate::db::inmemory_queue::InMemoryQueueRepository;
use crate::db::inmemory_rarity::InMemoryRarityRepository;
use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
use crate::db::inmemory_user::InMemoryUserRepository;
use crate::event::NoopEventPublisher;
use crate::random::StandartRandomProvider;

#[derive(Clone)]
pub struct AppState {
    pub slot_repo: Arc<InMemoryRouletteSlotRepository>,
    pub rarity_repo: Arc<InMemoryRarityRepository>,
    pub user_repo: Arc<InMemoryUserRepository>,
    pub platform_repo: Arc<InMemoryPlatformRepository>,
    pub queue_repo: Arc<InMemoryQueueRepository>,
    pub random: StandartRandomProvider,
    pub event_publisher: NoopEventPublisher,
}

impl AppState {
    pub fn new(
        slot_repo: InMemoryRouletteSlotRepository,
        rarity_repo: InMemoryRarityRepository,
        random: StandartRandomProvider,
    ) -> Self {
        Self {
            slot_repo: Arc::new(slot_repo),
            rarity_repo: Arc::new(rarity_repo),
            user_repo: Arc::new(InMemoryUserRepository::new()),
            platform_repo: Arc::new(InMemoryPlatformRepository::new_seeded()),
            queue_repo: Arc::new(InMemoryQueueRepository::new()),
            random,
            event_publisher: NoopEventPublisher,
        }
    }
}
