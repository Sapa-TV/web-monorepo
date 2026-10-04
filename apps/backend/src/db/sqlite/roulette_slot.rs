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
#[path = "roulette_slot.test.rs"]
mod tests;
