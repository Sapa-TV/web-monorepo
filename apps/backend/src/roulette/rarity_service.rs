use std::sync::nonpoison::RwLock;

use super::rarity::{Rarity, RarityId, RarityRepository};
use crate::error::RepositoryError;

#[non_exhaustive]
pub struct RarityService<R: RarityRepository> {
    repo: R,
    rarities: RwLock<Vec<Rarity>>,
}

impl<R: RarityRepository> RarityService<R> {
    pub async fn build(repo: R) -> Result<Self, RepositoryError> {
        let rarities = repo.load_all().await?;
        Ok(Self {
            repo,
            rarities: RwLock::new(rarities),
        })
    }

    pub fn get_all(&self) -> Vec<Rarity> {
        self.rarities.read().clone()
    }

    pub fn get_by_id(&self, id: RarityId) -> Option<Rarity> {
        self.rarities.read().iter().find(|r| r.id == id).cloned()
    }

    pub async fn save(&self, rarity: Rarity) -> Result<Rarity, RepositoryError> {
        let saved = self.repo.save(rarity).await?;
        self.rarities.write().push(saved.clone());
        Ok(saved)
    }

    pub async fn update(&self, rarity: Rarity) -> Result<Option<Rarity>, RepositoryError> {
        let Some(updated) = self.repo.update(rarity).await? else {
            return Ok(None);
        };
        if let Some(existing) = self
            .rarities
            .write()
            .iter_mut()
            .find(|r| r.id == updated.id)
        {
            *existing = updated.clone();
        }
        Ok(Some(updated))
    }

    pub async fn delete(&self, id: RarityId) -> Result<bool, RepositoryError> {
        let deleted = self.repo.delete(id).await?;
        self.rarities.write().retain(|r| r.id != id);
        Ok(deleted)
    }
}

#[cfg(test)]
#[path = "rarity_service.test.rs"]
mod tests;
