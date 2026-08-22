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
mod tests {
    use crate::db::sqlite::test_pool;

    use super::*;

    async fn repo() -> SqlitePlatformRepository {
        let (pool, _path) = test_pool().await;
        SqlitePlatformRepository::new(pool)
    }

    #[tokio::test]
    async fn load_all_returns_seeded_platforms() {
        let repo = repo().await;

        let all = repo.load_all().await.unwrap();

        assert_eq!(all.len(), 3);
        assert_eq!(all[0].id, PlatformId::TWITCH);
        assert_eq!(all[0].as_name(), "twitch");
        assert_eq!(all[1].id, PlatformId::YOUTUBE);
        assert_eq!(all[2].id, PlatformId::VK_VIDEO_LIVE);
    }

    #[tokio::test]
    async fn find_by_name_matches_seed() {
        let repo = repo().await;

        let twitch = repo.find_by_name("twitch").await.unwrap().unwrap();
        assert_eq!(twitch.id, PlatformId::TWITCH);

        assert!(repo.find_by_name("tiktok").await.unwrap().is_none());
    }
}
