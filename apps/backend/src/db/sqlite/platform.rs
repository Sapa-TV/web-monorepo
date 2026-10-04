use sqlx::SqlitePool;

use crate::db::sqlite::map_err;
use crate::error::RepositoryError;
use crate::platform::{Platform, PlatformId, PlatformRepository};

#[non_exhaustive]
pub struct SqlitePlatformRepository {
    pool: SqlitePool,
}

impl SqlitePlatformRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl PlatformRepository for SqlitePlatformRepository {
    async fn find_by_name(&self, name: &str) -> Result<Option<Platform>, RepositoryError> {
        let row = sqlx::query!("SELECT id, name FROM platforms WHERE name = ?", name)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(row.map(|row| Platform::new(PlatformId::new(row.id as u32), row.name)))
    }

    async fn load_all(&self) -> Result<Vec<Platform>, RepositoryError> {
        let rows = sqlx::query!("SELECT id, name FROM platforms ORDER BY id")
            .fetch_all(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|row| Platform::new(PlatformId::new(row.id as u32), row.name))
            .collect())
    }
}

#[cfg(test)]
#[path = "platform.test.rs"]
mod tests;
