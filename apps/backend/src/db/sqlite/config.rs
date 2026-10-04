use sqlx::SqlitePool;

use crate::config::repository::ConfigRepository;
use crate::config::runtime::RuntimeConfig;
use crate::db::sqlite::map_err;
use crate::error::RepositoryError;

#[non_exhaustive]
pub struct SqliteConfigRepository {
    pool: SqlitePool,
}

impl SqliteConfigRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl ConfigRepository for SqliteConfigRepository {
    async fn load(&self) -> Result<Option<RuntimeConfig>, RepositoryError> {
        let row = sqlx::query!("SELECT payload FROM runtime_config WHERE id = 1")
            .fetch_optional(&self.pool)
            .await
            .map_err(map_err)?;
        match row {
            Some(row) => serde_json::from_str(&row.payload)
                .map(Some)
                .map_err(|e| RepositoryError::Database(format!("invalid runtime config: {e}"))),
            None => Ok(None),
        }
    }

    async fn save(&self, config: &RuntimeConfig) -> Result<(), RepositoryError> {
        let payload = serde_json::to_string(config)
            .map_err(|e| RepositoryError::Database(format!("serialize config failed: {e}")))?;
        sqlx::query!(
            "INSERT INTO runtime_config (id, payload) VALUES (1, ?)
             ON CONFLICT(id) DO UPDATE SET payload = excluded.payload",
            payload
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "config.test.rs"]
mod tests;
