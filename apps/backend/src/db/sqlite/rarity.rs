use sqlx::SqlitePool;

use crate::db::sqlite::map_err;
use crate::error::RepositoryError;
use crate::roulette::rarity::{Rarity, RarityId, RarityRepository};

/// FK-enforced: deleting a rarity cascades to its roulette slots.
#[non_exhaustive]
pub struct SqliteRarityRepository {
    pool: SqlitePool,
}

impl SqliteRarityRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl RarityRepository for SqliteRarityRepository {
    async fn load_all(&self) -> Result<Vec<Rarity>, RepositoryError> {
        let rows =
            sqlx::query!("SELECT id, name, display_name, image, color FROM rarities ORDER BY id")
                .fetch_all(&self.pool)
                .await
                .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|row| {
                Rarity::new(
                    RarityId::new(row.id as u32),
                    row.name,
                    row.display_name,
                    row.image,
                    row.color,
                )
            })
            .collect())
    }

    async fn save(&self, rarity: Rarity) -> Result<Rarity, RepositoryError> {
        let row = sqlx::query!(
            "INSERT INTO rarities (name, display_name, image, color) VALUES (?, ?, ?, ?)
             RETURNING id",
            rarity.name,
            rarity.display_name,
            rarity.image,
            rarity.color
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(Rarity::new(
            RarityId::new(row.id as u32),
            rarity.name,
            rarity.display_name,
            rarity.image,
            rarity.color,
        ))
    }

    async fn update(&self, rarity: Rarity) -> Result<Option<Rarity>, RepositoryError> {
        let row = sqlx::query!(
            "UPDATE rarities SET name = ?1, display_name = ?2, image = ?3, color = ?4
             WHERE id = ?5
             RETURNING id, name, display_name, image, color",
            rarity.name,
            rarity.display_name,
            rarity.image,
            rarity.color,
            rarity.id.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|row| {
            Rarity::new(
                RarityId::new(row.id as u32),
                row.name,
                row.display_name,
                row.image,
                row.color,
            )
        }))
    }

    async fn delete(&self, id: RarityId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!("DELETE FROM rarities WHERE id = ?", id.get())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::sqlite::test_pool;

    use super::*;

    async fn repo() -> SqliteRarityRepository {
        let (pool, _path) = test_pool().await;
        SqliteRarityRepository::new(pool)
    }

    #[tokio::test]
    async fn load_all_returns_seeded_four_in_order() {
        let repo = repo().await;

        let all = repo.load_all().await.unwrap();

        let ids: Vec<u32> = all.iter().map(|r| r.id.get()).collect();
        assert_eq!(ids, vec![1, 2, 3, 4]);
        assert_eq!(all[0].name, "common");
        assert_eq!(all[3].name, "legendary");
    }

    #[tokio::test]
    async fn save_assigns_next_id_after_seed() {
        let repo = repo().await;

        let saved = repo
            .save(Rarity::new(
                RarityId::new(0),
                "mythic",
                "Mythic",
                "mythic.png",
                "#733f88",
            ))
            .await
            .unwrap();

        assert_eq!(saved.id.get(), 5);
        assert_eq!(saved.name, "mythic");
    }

    #[tokio::test]
    async fn update_roundtrip_and_missing_is_none() {
        let repo = repo().await;

        let updated = repo
            .update(Rarity::new(
                RarityId::new(1),
                "common",
                "Common Renamed",
                "common.png",
                "#ffffff",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.display_name, "Common Renamed");
        assert_eq!(updated.id.get(), 1);

        assert!(
            repo.update(Rarity::new(
                RarityId::new(999),
                "x",
                "X",
                "x.png",
                "#000000"
            ))
            .await
            .unwrap()
            .is_none()
        );
    }

    #[tokio::test]
    async fn delete_reports_presence() {
        let repo = repo().await;

        assert!(repo.delete(RarityId::new(4)).await.unwrap());
        assert!(!repo.delete(RarityId::new(4)).await.unwrap());
    }

    #[tokio::test]
    async fn deleting_rarity_cascades_to_its_slots() {
        use crate::db::sqlite::roulette_slot::SqliteRouletteSlotRepository;
        use crate::roulette::repository::RouletteSlotRepository;

        let (pool, _path) = test_pool().await;
        let repo = SqliteRarityRepository::new(pool.clone());
        let slots = SqliteRouletteSlotRepository::new(pool);

        assert_eq!(slots.load_all().await.unwrap().len(), 4);

        assert!(repo.delete(RarityId::new(4)).await.unwrap());

        let remaining = slots.load_all().await.unwrap();
        assert_eq!(remaining.len(), 3);
        assert!(remaining.iter().all(|s| s.name != "Джекпот"));
    }
}
