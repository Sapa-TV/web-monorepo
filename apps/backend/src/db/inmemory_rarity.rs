use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::nonpoison::Mutex;

use crate::error::RepositoryError;
use crate::roulette::rarity::{Rarity, RarityId, RarityRepository};

#[non_exhaustive]
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

    pub fn new_seeded() -> Self {
        let rarities = vec![
            Rarity::new(
                RarityId::new(1),
                "common",
                "Common",
                "common.png",
                "#9d9d9d",
            ),
            Rarity::new(RarityId::new(2), "rare", "Rare", "rare.png", "#4CAF50"),
            Rarity::new(RarityId::new(3), "epic", "Epic", "epic.png", "#9C27B0"),
            Rarity::new(
                RarityId::new(4),
                "legendary",
                "Legendary",
                "legendary.png",
                "#FFD700",
            ),
        ];
        Self {
            rarities: Mutex::new(rarities),
            next_id: AtomicU32::new(5),
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

    async fn update(&self, rarity: Rarity) -> Result<Option<Rarity>, RepositoryError> {
        let mut rarities = self.rarities.lock();
        if let Some(existing) = rarities.iter_mut().find(|r| r.id == rarity.id) {
            *existing = rarity.clone();
            Ok(Some(rarity))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, id: RarityId) -> Result<bool, RepositoryError> {
        let mut rarities = self.rarities.lock();
        let len_before = rarities.len();
        rarities.retain(|r| r.id != id);
        Ok(rarities.len() != len_before)
    }
}
