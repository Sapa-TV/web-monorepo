use std::sync::Arc;

use crate::db::inmemory_platform::InMemoryPlatformRepository;
use crate::db::inmemory_rarity::InMemoryRarityRepository;
use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;
use crate::db::inmemory_user::InMemoryUserRepository;
use crate::random::StandartRandomProvider;

#[derive(Clone)]
pub struct AppState {
    pub slot_repo: Arc<InMemoryRouletteSlotRepository>,
    pub rarity_repo: Arc<InMemoryRarityRepository>,
    pub user_repo: Arc<InMemoryUserRepository>,
    pub platform_repo: Arc<InMemoryPlatformRepository>,
    pub random: StandartRandomProvider,
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
            random,
        }
    }
}