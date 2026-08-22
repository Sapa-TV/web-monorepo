use sqlx::SqlitePool;

use crate::db::sqlite::map_err;
use crate::error::RepositoryError;
use crate::roulette::rarity::RarityId;
use crate::roulette::repository::RouletteSlotRepository;
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};

#[non_exhaustive]
pub struct SqliteRouletteSlotRepository {
    pool: SqlitePool,
}

impl SqliteRouletteSlotRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn weight_to_db(weight: u64) -> Result<i64, RepositoryError> {
    i64::try_from(weight).map_err(|_| RepositoryError::Conflict("weight out of range".to_string()))
}

impl RouletteSlotRepository for SqliteRouletteSlotRepository {
    async fn load_all(&self) -> Result<Vec<RouletteSlot>, RepositoryError> {
        let rows = sqlx::query!(
            "SELECT id, name, rarity_id, weight, action FROM roulette_slots ORDER BY id"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|row| {
                RouletteSlot::new(
                    RouletteSlotId::new(row.id as u32),
                    row.name,
                    RarityId::new(row.rarity_id as u32),
                    row.weight as u64,
                    row.action,
                )
            })
            .collect())
    }

    async fn save(&self, slot: RouletteSlot) -> Result<RouletteSlot, RepositoryError> {
        let weight = weight_to_db(slot.weight)?;
        let row = sqlx::query!(
            "INSERT INTO roulette_slots (name, rarity_id, weight, action) VALUES (?, ?, ?, ?)
             RETURNING id AS \"id!: i64\"",
            slot.name,
            slot.rarity_id.get(),
            weight,
            slot.action
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(RouletteSlot::new(
            RouletteSlotId::new(row.id as u32),
            slot.name,
            slot.rarity_id,
            slot.weight,
            slot.action,
        ))
    }

    async fn update(&self, slot: RouletteSlot) -> Result<Option<RouletteSlot>, RepositoryError> {
        let weight = weight_to_db(slot.weight)?;
        let row = sqlx::query!(
            "UPDATE roulette_slots SET name = ?1, rarity_id = ?2, weight = ?3, action = ?4
             WHERE id = ?5
             RETURNING id AS \"id!: i64\"",
            slot.name,
            slot.rarity_id.get(),
            weight,
            slot.action,
            slot.id.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|row| {
            RouletteSlot::new(
                RouletteSlotId::new(row.id as u32),
                slot.name.clone(),
                slot.rarity_id,
                slot.weight,
                slot.action.clone(),
            )
        }))
    }

    async fn delete(&self, id: RouletteSlotId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!("DELETE FROM roulette_slots WHERE id = ?", id.get())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::sqlite::test_pool;
    use crate::error::RepositoryError;
    use crate::roulette::rarity::RarityId;
    use crate::roulette::slot_service::RouletteSlotId;

    use super::*;

    async fn repo() -> SqliteRouletteSlotRepository {
        let (pool, _path) = test_pool().await;
        SqliteRouletteSlotRepository::new(pool)
    }

    #[tokio::test]
    async fn load_all_returns_seeded_slots_in_order() {
        let repo = repo().await;

        let all = repo.load_all().await.unwrap();

        assert_eq!(all.len(), 4);
        assert_eq!(all[0].name, "Поболтать");
        assert_eq!(all[0].weight, 50);
        assert_eq!(all[3].name, "Джекпот");
        assert_eq!(all[3].weight, 1);
        assert_eq!(all[3].rarity_id, RarityId::new(4));
    }

    #[tokio::test]
    async fn save_assigns_next_id_and_roundtrips() {
        let repo = repo().await;

        let saved = repo
            .save(RouletteSlot::new(
                RouletteSlotId::new(999),
                "Новый",
                RarityId::new(2),
                7,
                "chat",
            ))
            .await
            .unwrap();

        assert_eq!(saved.id.get(), 5);

        let all = repo.load_all().await.unwrap();
        let fetched = all.iter().find(|s| s.id == saved.id).unwrap();
        assert_eq!(fetched.name, "Новый");
        assert_eq!(fetched.weight, 7);
    }

    #[tokio::test]
    async fn update_changes_fields_and_missing_is_none() {
        let repo = repo().await;

        let updated = repo
            .update(RouletteSlot::new(
                RouletteSlotId::new(1),
                "Поболтать",
                RarityId::new(3),
                99,
                "superchat",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.weight, 99);
        assert_eq!(updated.rarity_id, RarityId::new(3));

        assert!(
            repo.update(RouletteSlot::new(
                RouletteSlotId::new(999),
                "x",
                RarityId::new(1),
                1,
                "chat"
            ))
            .await
            .unwrap()
            .is_none()
        );
    }

    #[tokio::test]
    async fn delete_reports_presence() {
        let repo = repo().await;

        assert!(repo.delete(RouletteSlotId::new(4)).await.unwrap());
        assert!(!repo.delete(RouletteSlotId::new(4)).await.unwrap());
    }

    #[tokio::test]
    async fn weight_above_i64_max_is_conflict_not_panic() {
        let repo = repo().await;

        let err = repo
            .save(RouletteSlot::new(
                RouletteSlotId::new(999),
                "huge",
                RarityId::new(1),
                (i64::MAX as u64) + 1,
                "chat",
            ))
            .await
            .unwrap_err();

        match err {
            RepositoryError::Conflict(m) => assert_eq!(m, "weight out of range"),
            other => panic!("expected Conflict, got {other:?}"),
        }

        let err_update = repo
            .update(RouletteSlot::new(
                RouletteSlotId::new(1),
                "huge",
                RarityId::new(1),
                u64::MAX,
                "chat",
            ))
            .await
            .unwrap_err();
        assert!(matches!(err_update, RepositoryError::Conflict(_)));

        assert_eq!(repo.load_all().await.unwrap().len(), 4);
    }
}
