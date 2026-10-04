use std::sync::Arc;

use crate::roulette::repository::RouletteSlotRepository;
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotService};

pub trait RandomProvider {
    fn next(&self) -> f64;
}

#[derive(Clone)]
#[non_exhaustive]
pub struct RouletteService<Rand: RandomProvider, Repo: RouletteSlotRepository> {
    slot_service: Arc<RouletteSlotService<Repo>>,
    random: Rand,
}

impl<Rand: RandomProvider, Repo: RouletteSlotRepository> RouletteService<Rand, Repo> {
    pub fn new(slot_service: Arc<RouletteSlotService<Repo>>, random: Rand) -> Self {
        Self {
            slot_service,
            random,
        }
    }

    pub fn roll(&self) -> Option<RouletteSlot> {
        let total_weight: u64 = self.slot_service.total_weight();
        let random_value = self.random.next() * total_weight as f64;

        self.slot_service.get_slot_by_weight(random_value as u64)
    }
}

#[cfg(test)]
#[path = "machine.test.rs"]
mod tests;
