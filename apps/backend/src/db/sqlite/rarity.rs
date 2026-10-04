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
#[path = "rarity.test.rs"]
mod tests;
