use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::nonpoison::Mutex;

use crate::error::RepositoryError;
use crate::roulette::rarity::RarityId;
use crate::roulette::repository::RouletteSlotRepository;
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};

#[non_exhaustive]
pub struct InMemoryRouletteSlotRepository {
    slots: Mutex<Vec<RouletteSlot>>,
    next_id: AtomicU32,
}

const SEEDED_SLOTS: &[(&str, u32, u64, &str)] = &[
    ("Поболтать", 1, 50, "chat"),
    ("Подписка", 2, 20, "subscribe"),
    ("Суперчат", 3, 5, "superchat"),
    ("Джекпот", 4, 1, "jackpot"),
];

impl Default for InMemoryRouletteSlotRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryRouletteSlotRepository {
    pub fn new() -> Self {
        Self {
            slots: Mutex::new(Vec::new()),
            next_id: AtomicU32::new(1),
        }
    }

    pub fn new_seeded() -> Self {
        let slots: Vec<RouletteSlot> = SEEDED_SLOTS
            .iter()
            .enumerate()
            .map(|(i, (name, rarity, weight, action))| {
                RouletteSlot::new(
                    RouletteSlotId::new(i as u32 + 1),
                    *name,
                    RarityId::new(*rarity),
                    *weight,
                    *action,
                )
            })
            .collect();
        let next_id = slots.len() as u32 + 1;
        Self {
            slots: Mutex::new(slots),
            next_id: AtomicU32::new(next_id),
        }
    }

    #[allow(dead_code)]
    pub fn seed(slots: Vec<RouletteSlot>) -> Self {
        let assigned: Vec<RouletteSlot> = slots
            .into_iter()
            .enumerate()
            .map(|(i, slot)| {
                RouletteSlot::new(
                    RouletteSlotId::new(i as u32 + 1),
                    &slot.name,
                    slot.rarity_id,
                    slot.weight,
                    &slot.action,
                )
            })
            .collect();
        let next_id = assigned.len() as u32 + 1;
        Self {
            slots: Mutex::new(assigned),
            next_id: AtomicU32::new(next_id),
        }
    }
}

impl RouletteSlotRepository for InMemoryRouletteSlotRepository {
    async fn load_all(&self) -> Result<Vec<RouletteSlot>, RepositoryError> {
        Ok(self.slots.lock().clone())
    }

    async fn save(&self, mut slot: RouletteSlot) -> Result<RouletteSlot, RepositoryError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        slot.id = RouletteSlotId::new(id);
        self.slots.lock().push(slot.clone());
        Ok(slot)
    }

    async fn update(&self, slot: RouletteSlot) -> Result<Option<RouletteSlot>, RepositoryError> {
        let mut slots = self.slots.lock();
        if let Some(existing) = slots.iter_mut().find(|s| s.id == slot.id) {
            *existing = slot.clone();
            Ok(Some(slot))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, id: RouletteSlotId) -> Result<bool, RepositoryError> {
        let mut slots = self.slots.lock();
        let len_before = slots.len();
        slots.retain(|s| s.id != id);
        Ok(slots.len() != len_before)
    }
}

#[cfg(test)]
#[path = "inmemory_roulette_slots.test.rs"]
mod tests;
