use super::repository::RouletteSlotRepository;
use crate::error::RepositoryError;
use crate::roulette::rarity::RarityId;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use std::sync::nonpoison::RwLock;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct RouletteSlotId(u32);

impl RouletteSlotId {
    pub(crate) fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u32 {
        self.0
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
    #[serde(skip)]
    _sealed: (),
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
            _sealed: (),
        }
    }
}

#[non_exhaustive]
pub struct RouletteSlotService<R: RouletteSlotRepository> {
    repo: R,
    slots: RwLock<Vec<RouletteSlot>>,
}

impl<R: RouletteSlotRepository> RouletteSlotService<R> {
    pub async fn build(repo: R) -> Result<Self, RepositoryError> {
        let slots = repo.load_all().await?;
        Ok(Self {
            repo,
            slots: RwLock::new(slots),
        })
    }

    pub fn get_slots(&self) -> Vec<RouletteSlot> {
        self.slots.read().clone()
    }

    pub fn get_slot_by_id(&self, id: RouletteSlotId) -> Option<RouletteSlot> {
        self.slots.read().iter().find(|s| s.id == id).cloned()
    }

    pub fn get_name(&self, id: RouletteSlotId) -> Option<String> {
        self.get_slot_by_id(id).map(|s| s.name)
    }

    pub fn total_weight(&self) -> u64 {
        self.slots.read().iter().map(|slot| slot.weight).sum()
    }

    pub fn get_slot_by_weight(&self, weight: u64) -> Option<RouletteSlot> {
        let slots = self.slots.read();
        let mut current_weight = 0;
        for slot in slots.iter() {
            current_weight += slot.weight;
            if weight < current_weight {
                return Some(slot.clone());
            }
        }

        slots.iter().max_by_key(|slot| slot.weight).cloned()
    }

    pub async fn add_slot(&self, slot: RouletteSlot) -> Result<RouletteSlot, RepositoryError> {
        let saved = self.repo.save(slot).await?;
        self.slots.write().push(saved.clone());
        Ok(saved)
    }

    pub async fn edit_slot(
        &self,
        slot: RouletteSlot,
    ) -> Result<Option<RouletteSlot>, RepositoryError> {
        let Some(updated) = self.repo.update(slot).await? else {
            return Ok(None);
        };
        if let Some(existing) = self.slots.write().iter_mut().find(|s| s.id == updated.id) {
            *existing = updated.clone();
        }
        Ok(Some(updated))
    }

    pub async fn delete_slot(&self, id: RouletteSlotId) -> Result<bool, RepositoryError> {
        let deleted = self.repo.delete(id).await?;
        self.slots.write().retain(|s| s.id != id);
        Ok(deleted)
    }
}

#[cfg(test)]
#[path = "slot_service.test.rs"]
mod tests;
