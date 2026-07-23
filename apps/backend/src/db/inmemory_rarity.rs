use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::nonpoison::Mutex;

use crate::error::RepositoryError;
use crate::roulette::rarity::{Rarity, RarityId, RarityRepository};

pub struct InMemoryRarityRepository {
    rarities: Mutex<Vec<Rarity>>,
    next_id: AtomicU32,
}

impl InMemoryRarityRepository {
    pub fn new() -> Self {
        Self {
            rarities: Mutex::new(Vec::new()),
            next_id: AtomicU32::new(1),
        }
    }

    #[allow(dead_code)]
    pub fn seed(rarities: Vec<Rarity>) -> Self {
        let assigned: Vec<Rarity> = rarities
            .into_iter()
            .enumerate()
            .map(|(i, r)| {
                Rarity::new(
                    RarityId::new(i as u32 + 1),
                    &r.name,
                    &r.display_name,
                    &r.image,
                    &r.color,
                )
            })
            .collect();
        let next_id = assigned.len() as u32 + 1;
        Self {
            rarities: Mutex::new(assigned),
            next_id: AtomicU32::new(next_id),
        }
    }
}

impl RarityRepository for InMemoryRarityRepository {
    async fn load_all(&self) -> Result<Vec<Rarity>, RepositoryError> {
        Ok(self.rarities.lock().clone())
    }

    async fn save(&self, mut rarity: Rarity) -> Result<Rarity, RepositoryError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        rarity.id = RarityId::new(id);
        self.rarities.lock().push(rarity.clone());
        Ok(rarity)
    }

    async fn update(&self, rarity: Rarity) -> Result<Rarity, RepositoryError> {
        let mut rarities = self.rarities.lock();
        if let Some(existing) = rarities.iter_mut().find(|r| r.id == rarity.id) {
            *existing = rarity.clone();
            Ok(rarity)
        } else {
            Err(RepositoryError::NotFound(rarity.id.value()))
        }
    }

    async fn delete(&self, id: RarityId) -> Result<(), RepositoryError> {
        let mut rarities = self.rarities.lock();
        let len_before = rarities.len();
        rarities.retain(|r| r.id != id);
        if rarities.len() == len_before {
            Err(RepositoryError::NotFound(id.value()))
        } else {
            Ok(())
        }
    }
}
