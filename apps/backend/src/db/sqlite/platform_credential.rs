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
mod tests {
    use crate::db::sqlite::test_pool;
    use crate::platform::PlatformId;

    use super::*;

    async fn repo() -> SqlitePlatformCredentialRepository {
        let (pool, _path) = test_pool().await;
        SqlitePlatformCredentialRepository::new(pool)
    }

    #[tokio::test]
    async fn load_on_missing_is_none() {
        let repo = repo().await;
        assert!(repo.load_credential(PlatformId::TWITCH).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn save_trims_and_roundtrips() {
        let repo = repo().await;

        repo.save_credential(PlatformId::TWITCH, "  tok-1  ")
            .await
            .unwrap();

        assert_eq!(
            repo.load_credential(PlatformId::TWITCH).await.unwrap(),
            Some("tok-1".to_string())
        );
    }

    #[tokio::test]
    async fn save_overwrites_previous() {
        let repo = repo().await;

        repo.save_credential(PlatformId::TWITCH, "old").await.unwrap();
        repo.save_credential(PlatformId::TWITCH, "new").await.unwrap();

        assert_eq!(
            repo.load_credential(PlatformId::TWITCH).await.unwrap(),
            Some("new".to_string())
        );
        assert!(repo.load_credential(PlatformId::YOUTUBE).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn clear_is_idempotent() {
        let repo = repo().await;

        repo.save_credential(PlatformId::VK_VIDEO_LIVE, "tok").await.unwrap();
        repo.clear_credential(PlatformId::VK_VIDEO_LIVE).await.unwrap();

        assert!(repo.load_credential(PlatformId::VK_VIDEO_LIVE).await.unwrap().is_none());
        repo.clear_credential(PlatformId::VK_VIDEO_LIVE).await.unwrap();
    }
}
