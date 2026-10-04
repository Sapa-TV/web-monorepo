use sqlx::SqlitePool;

use crate::db::sqlite::map_err;
use crate::error::RepositoryError;
use crate::platform::{PlatformCredentialRepository, PlatformId};

#[non_exhaustive]
pub struct SqlitePlatformCredentialRepository {
    pool: SqlitePool,
}

impl SqlitePlatformCredentialRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl PlatformCredentialRepository for SqlitePlatformCredentialRepository {
    async fn load_credential(
        &self,
        platform: PlatformId,
    ) -> Result<Option<String>, RepositoryError> {
        let row = sqlx::query!(
            "SELECT credential FROM platform_credentials WHERE platform_id = ?",
            platform.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|row| row.credential))
    }

    async fn save_credential(
        &self,
        platform: PlatformId,
        credential: &str,
    ) -> Result<(), RepositoryError> {
        let trimmed = credential.trim().to_string();
        sqlx::query!(
            "INSERT INTO platform_credentials (platform_id, credential) VALUES (?, ?)
             ON CONFLICT(platform_id) DO UPDATE SET credential = excluded.credential",
            platform.get(),
            trimmed
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn clear_credential(&self, platform: PlatformId) -> Result<(), RepositoryError> {
        sqlx::query!(
            "DELETE FROM platform_credentials WHERE platform_id = ?",
            platform.get()
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "platform_credential.test.rs"]
mod tests;
