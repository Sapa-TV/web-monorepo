use super::repository::RouletteSlotRepository;
use crate::error::RepositoryError;
use crate::roulette::rarity::RarityId;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct RouletteSlotId(u32);

impl RouletteSlotId {
    pub(crate) fn new(id: u32) -> Self {
        Self(id)
    }
}

impl Display for RouletteSlotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
#[non_exhaustive]
pub struct RouletteSlot {
    pub(crate) id: RouletteSlotId,
    pub(crate) name: String,
    pub(crate) rarity_id: RarityId,
    pub(crate) weight: u64,
    pub(crate) action: String,
}

impl RouletteSlot {
    pub fn new<S1, S2>(
        id: RouletteSlotId,
        name: S1,
        rarity_id: RarityId,
        weight: u64,
        action: S2,
    ) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self {
            id,
            name: name.into(),
            rarity_id,
            weight,
            action: action.into(),
        }
    }
}

#[non_exhaustive]
pub struct RouletteSlotService<R: RouletteSlotRepository> {
    repo: R,
    slots: Vec<RouletteSlot>,
}

impl<R: RouletteSlotRepository> RouletteSlotService<R> {
    pub async fn build(repo: R) -> Result<Self, RepositoryError> {
        let slots = repo.load_all().await?;
        Ok(Self { repo, slots })
    }

    pub fn get_slots(&self) -> &[RouletteSlot] {
        &self.slots
    }

    pub fn total_weight(&self) -> u64 {
        self.slots.iter().map(|slot| slot.weight).sum()
    }

    pub fn get_slot_by_weight(&self, weight: u64) -> Option<&RouletteSlot> {
        let mut current_weight = 0;
        for slot in &self.slots {
            current_weight += slot.weight;
            if weight < current_weight {
                return Some(slot);
            }
        }

        self.slots.iter().max_by_key(|slot| slot.weight)
    }

    pub async fn add_slot(&mut self, slot: RouletteSlot) -> Result<(), RepositoryError> {
        let saved = self.repo.save(slot).await?;
        self.slots.push(saved);
        Ok(())
    }

    pub async fn edit_slot(&mut self, slot: RouletteSlot) -> Result<(), RepositoryError> {
        if let Some(updated) = self.repo.update(slot).await?
            && let Some(existing) = self.slots.iter_mut().find(|s| s.id == updated.id)
        {
            *existing = updated;
        }
        Ok(())
    }

    pub async fn delete_slot(&mut self, id: RouletteSlotId) -> Result<(), RepositoryError> {
        self.repo.delete(id).await?;
        self.slots.retain(|s| s.id != id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::inmemory_roulette_slots::InMemoryRouletteSlotRepository;

    const COMMON: RarityId = RarityId::new(1);

    use super::*;

    #[tokio::test]
    async fn test_total_weight_calc() {
        let slots = vec![
            RouletteSlot::new(RouletteSlotId::new(0), "Test_1", COMMON, 123, "Action 1"),
            RouletteSlot::new(RouletteSlotId::new(0), "Test_2", COMMON, 246, "Action 2"),
        ];
        let repo = InMemoryRouletteSlotRepository::seed(slots);
        let slot_service = RouletteSlotService::build(repo).await.unwrap();

        let total_weight = slot_service.total_weight();
        assert_eq!(total_weight, 369);
    }

    #[tokio::test]
    async fn test_mid_boundary_switching() {
        let slots = vec![
            RouletteSlot::new(RouletteSlotId::new(0), "Test_1", COMMON, 10, "Action 1"),
            RouletteSlot::new(RouletteSlotId::new(0), "Test_2", COMMON, 20, "Action 2"),
        ];
        let repo = InMemoryRouletteSlotRepository::seed(slots);
        let slot_service = RouletteSlotService::build(repo).await.unwrap();

        let slot = slot_service.get_slot_by_weight(0).unwrap();
        assert_eq!(slot.name, "Test_1".to_string());

        let slot = slot_service.get_slot_by_weight(9).unwrap();
        assert_eq!(slot.name, "Test_1".to_string());

        let slot = slot_service.get_slot_by_weight(10).unwrap();
        assert_eq!(slot.name, "Test_2".to_string());
    }

    #[tokio::test]
    async fn test_absolute_favorite() {
        let slots = vec![
            RouletteSlot::new(RouletteSlotId::new(0), "Loser_1", COMMON, 0, "Loser 1"),
            RouletteSlot::new(RouletteSlotId::new(0), "Winner", COMMON, 20, "Winner"),
            RouletteSlot::new(RouletteSlotId::new(0), "Loser_2", COMMON, 0, "Loser 1"),
        ];
        let repo = InMemoryRouletteSlotRepository::seed(slots);
        let slot_service = RouletteSlotService::build(repo).await.unwrap();

        let slot = slot_service.get_slot_by_weight(0).unwrap();
        assert_eq!(slot.name, "Winner".to_string());

        let slot = slot_service.get_slot_by_weight(10).unwrap();
        assert_eq!(slot.name, "Winner".to_string());

        let slot = slot_service.get_slot_by_weight(19).unwrap();
        assert_eq!(slot.name, "Winner".to_string());
    }

    #[tokio::test]
    async fn test_fallback_heaviest() {
        let slots = vec![
            RouletteSlot::new(RouletteSlotId::new(0), "Loser_1", COMMON, 0, "Loser 1"),
            RouletteSlot::new(RouletteSlotId::new(0), "Winner", COMMON, 20, "Winner"),
            RouletteSlot::new(RouletteSlotId::new(0), "Loser_2", COMMON, 0, "Loser 1"),
        ];
        let repo = InMemoryRouletteSlotRepository::seed(slots);
        let slot_service = RouletteSlotService::build(repo).await.unwrap();

        let slot = slot_service.get_slot_by_weight(30).unwrap();
        assert_eq!(slot.name, "Winner".to_string());
    }

    #[tokio::test]
    async fn test_add_and_get_slots() {
        let repo = InMemoryRouletteSlotRepository::seed(vec![]);
        let mut slot_service = RouletteSlotService::build(repo).await.unwrap();

        slot_service
            .add_slot(RouletteSlot::new(
                RouletteSlotId::new(0),
                "New",
                COMMON,
                50,
                "Action",
            ))
            .await
            .unwrap();
        assert_eq!(slot_service.get_slots().len(), 1);
        assert_eq!(slot_service.total_weight(), 50);
    }

    #[tokio::test]
    async fn test_delete_slot() {
        let repo = InMemoryRouletteSlotRepository::seed(vec![]);
        let mut slot_service = RouletteSlotService::build(repo).await.unwrap();

        slot_service
            .add_slot(RouletteSlot::new(
                RouletteSlotId::new(0),
                "ToDelete",
                COMMON,
                10,
                "Act",
            ))
            .await
            .unwrap();
        let id = slot_service.get_slots()[0].id;
        slot_service.delete_slot(id).await.unwrap();
        assert!(slot_service.get_slots().is_empty());
    }

    #[tokio::test]
    async fn test_edit_slot() {
        let repo = InMemoryRouletteSlotRepository::seed(vec![]);
        let mut slot_service = RouletteSlotService::build(repo).await.unwrap();

        slot_service
            .add_slot(RouletteSlot::new(
                RouletteSlotId::new(0),
                "Original",
                COMMON,
                10,
                "Act",
            ))
            .await
            .unwrap();

        let id = slot_service.get_slots()[0].id;
        slot_service
            .edit_slot(RouletteSlot::new(id, "Edited", COMMON, 99, "NewAct"))
            .await
            .unwrap();

        assert_eq!(slot_service.get_slots()[0].name, "Edited");
        assert_eq!(slot_service.get_slots()[0].weight, 99);
    }
}
