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
mod tests {
    use crate::db::inmemory_rarity::InMemoryRarityRepository;

    use super::*;

    #[tokio::test]
    async fn build_loads_seeded() {
        let repo = InMemoryRarityRepository::new_seeded();
        let service = RarityService::build(repo).await.unwrap();
        assert_eq!(service.get_all().len(), 4);
    }

    #[tokio::test]
    async fn get_by_id_and_name() {
        let repo = InMemoryRarityRepository::new_seeded();
        let service = RarityService::build(repo).await.unwrap();

        assert_eq!(
            service
                .get_by_id(RarityId::new(1))
                .map(|r| r.display_name)
                .as_deref(),
            Some("Common")
        );
        assert!(service.get_by_id(RarityId::new(99)).is_none());
    }

    #[tokio::test]
    async fn save_update_delete_refresh_cache() {
        let repo = InMemoryRarityRepository::seed(vec![]);
        let service = RarityService::build(repo).await.unwrap();

        service
            .save(Rarity::new(
                RarityId::new(0),
                "c",
                "Custom",
                "c.png",
                "#fff",
            ))
            .await
            .unwrap();
        assert_eq!(service.get_all().len(), 1);

        let saved = service.get_all()[0].clone();
        service
            .update(Rarity::new(saved.id, "c", "Renamed", "c.png", "#fff"))
            .await
            .unwrap();
        assert_eq!(
            service
                .get_by_id(saved.id)
                .map(|r| r.display_name)
                .as_deref(),
            Some("Renamed")
        );

        service.delete(saved.id).await.unwrap();
        assert!(service.get_all().is_empty());
    }
}
