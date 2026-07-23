use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::nonpoison::Mutex;

use crate::error::RepositoryError;
use crate::roulette::repository::RouletteSlotRepository;
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};

pub struct InMemoryRouletteSlotRepository {
    slots: Mutex<Vec<RouletteSlot>>,
    next_id: AtomicU32,
}

impl InMemoryRouletteSlotRepository {
    pub fn new() -> Self {
        Self {
            slots: Mutex::new(Vec::new()),
            next_id: AtomicU32::new(1),
        }
    }

    pub fn seed(slots: Vec<RouletteSlot>) -> Self {
        let assigned: Vec<RouletteSlot> = slots
            .into_iter()
            .enumerate()
            .map(|(i, slot)| {
                RouletteSlot::new(
                    RouletteSlotId::new(i as u32 + 1),
                    &slot.name,
                    slot.rarity,
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

    async fn update(&self, slot: RouletteSlot) -> Result<RouletteSlot, RepositoryError> {
        let mut slots = self.slots.lock();
        if let Some(existing) = slots.iter_mut().find(|s| s.id == slot.id) {
            *existing = slot.clone();
            Ok(slot)
        } else {
            Err(RepositoryError::NotFound(slot.id.value()))
        }
    }

    async fn delete(&self, id: RouletteSlotId) -> Result<(), RepositoryError> {
        let mut slots = self.slots.lock();
        let len_before = slots.len();
        slots.retain(|s| s.id != id);
        if slots.len() == len_before {
            Err(RepositoryError::NotFound(id.value()))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::roulette::slot_service::RouletteSlotRarity::Common;

    use super::*;

    fn make_slot(name: &str) -> RouletteSlot {
        RouletteSlot::new(RouletteSlotId::new(0), name, Common, 10, "action")
    }

    #[tokio::test]
    async fn test_save_assigns_id() {
        let repo = InMemoryRouletteSlotRepository::new();
        let slot = make_slot("test");
        assert_eq!(slot.id.value(), 0);

        let saved = repo.save(slot).await.unwrap();
        assert_eq!(saved.id.value(), 1);
    }

    #[tokio::test]
    async fn test_save_increments_id() {
        let repo = InMemoryRouletteSlotRepository::new();
        let saved_1 = repo.save(make_slot("a")).await.unwrap();
        let saved_2 = repo.save(make_slot("b")).await.unwrap();
        assert_eq!(saved_1.id.value(), 1);
        assert_eq!(saved_2.id.value(), 2);
    }

    #[tokio::test]
    async fn test_load_all_returns_saved_slots() {
        let repo = InMemoryRouletteSlotRepository::new();
        repo.save(make_slot("a")).await.unwrap();
        repo.save(make_slot("b")).await.unwrap();
        repo.save(make_slot("c")).await.unwrap();

        let all = repo.load_all().await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_update_existing_slot() {
        let repo = InMemoryRouletteSlotRepository::new();
        let saved = repo.save(make_slot("original")).await.unwrap();

        let updated = RouletteSlot::new(saved.id, "updated", Common, 99, "new action");
        let result = repo.update(updated.clone()).await.unwrap();
        assert_eq!(result.name, "updated");
        assert_eq!(result.weight, 99);

        let all = repo.load_all().await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "updated");
    }

    #[tokio::test]
    async fn test_update_nonexistent_returns_error() {
        let repo = InMemoryRouletteSlotRepository::new();
        let slot = RouletteSlot::new(RouletteSlotId::new(999), "ghost", Common, 10, "action");
        let err = repo.update(slot).await.unwrap_err();
        assert_eq!(err, RepositoryError::NotFound(999));
    }

    #[tokio::test]
    async fn test_delete_existing_slot() {
        let repo = InMemoryRouletteSlotRepository::new();
        let saved = repo.save(make_slot("to_delete")).await.unwrap();
        assert_eq!(repo.load_all().await.unwrap().len(), 1);

        repo.delete(saved.id).await.unwrap();
        assert_eq!(repo.load_all().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_delete_nonexistent_returns_error() {
        let repo = InMemoryRouletteSlotRepository::new();
        let err = repo.delete(RouletteSlotId::new(42)).await.unwrap_err();
        assert_eq!(err, RepositoryError::NotFound(42));
    }

    #[tokio::test]
    async fn test_seed_load_all() {
        let slots = vec![make_slot("preloaded_a"), make_slot("preloaded_b")];
        let repo = InMemoryRouletteSlotRepository::seed(slots);
        let all = repo.load_all().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id.value(), 1);
        assert_eq!(all[1].id.value(), 2);
        assert_eq!(all[0].name, "preloaded_a");
    }

    #[tokio::test]
    async fn test_seed_continues_ids() {
        let slots = vec![make_slot("existing")];
        let repo = InMemoryRouletteSlotRepository::seed(slots);

        let saved = repo.save(make_slot("new")).await.unwrap();
        assert_eq!(saved.id.value(), 2);
    }

    #[tokio::test]
    async fn test_seed_normalizes_ids() {
        let s1 = RouletteSlot::new(RouletteSlotId::new(0), "a", Common, 10, "act");
        let s2 = RouletteSlot::new(RouletteSlotId::new(0), "b", Common, 10, "act");
        let repo = InMemoryRouletteSlotRepository::seed(vec![s1, s2]);

        let all = repo.load_all().await.unwrap();
        assert_eq!(all[0].id.value(), 1);
        assert_eq!(all[1].id.value(), 2);
    }
}
