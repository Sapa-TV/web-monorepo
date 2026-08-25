use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::db::sqlite::{conflict_on_unique, map_err};
use crate::error::RepositoryError;
use crate::platform::PlatformId;
use crate::user::repository::UserRepository;
use crate::user::{User, UserId, UserPlatform, UserPlatformId};

#[non_exhaustive]
pub struct SqliteUserRepository {
    pool: SqlitePool,
}

impl SqliteUserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl UserRepository for SqliteUserRepository {
    async fn create(&self, display_name: &str) -> Result<User, RepositoryError> {
        let now = Utc::now();
        let row = sqlx::query!(
            "INSERT INTO users (display_name, created_at, updated_at) VALUES (?, ?, ?)
             RETURNING id AS \"id!: i64\"",
            display_name,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(User::new(
            UserId::new(row.id as u32),
            display_name.to_string(),
            now,
            now,
        ))
    }

    async fn find_by_platform(
        &self,
        platform_id: PlatformId,
        platform_user_id: &str,
    ) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query!(
            r#"SELECT u.id AS "id!: i64", u.display_name,
                      u.created_at AS "created_at: DateTime<Utc>",
                      u.updated_at AS "updated_at: DateTime<Utc>"
               FROM users u
               JOIN user_platforms up ON up.user_id = u.id
               WHERE up.platform_id = ? AND up.platform_user_id = ?"#,
            platform_id.get(),
            platform_user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|row| {
            User::new(
                UserId::new(row.id as u32),
                row.display_name,
                row.created_at,
                row.updated_at,
            )
        }))
    }

    async fn get_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query!(
            r#"SELECT id AS "id!: i64", display_name,
                      created_at AS "created_at: DateTime<Utc>",
                      updated_at AS "updated_at: DateTime<Utc>"
               FROM users WHERE id = ?"#,
            id.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|row| {
            User::new(
                UserId::new(row.id as u32),
                row.display_name,
                row.created_at,
                row.updated_at,
            )
        }))
    }

    async fn get_platforms(&self, user_id: UserId) -> Result<Vec<UserPlatform>, RepositoryError> {
        let rows = sqlx::query!(
            r#"SELECT id AS "id!: i64", user_id AS "user_id!: i64",
                      platform_id AS "platform_id!: i64",
                      platform_user_id, platform_username
               FROM user_platforms WHERE user_id = ? ORDER BY id"#,
            user_id.get()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|row| {
                UserPlatform::new(
                    UserPlatformId::new(row.id as u32),
                    UserId::new(row.user_id as u32),
                    PlatformId::new(row.platform_id as u32),
                    row.platform_user_id,
                    row.platform_username,
                )
            })
            .collect())
    }

    async fn link_platform(
        &self,
        user_id: UserId,
        platform_id: PlatformId,
        platform_user_id: &str,
        platform_username: &str,
    ) -> Result<UserPlatform, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let link = sqlx::query!(
            "INSERT INTO user_platforms (user_id, platform_id, platform_user_id, platform_username)
             VALUES (?, ?, ?, ?)
             RETURNING id AS \"id!: i64\"",
            user_id.get(),
            platform_id.get(),
            platform_user_id,
            platform_username
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| conflict_on_unique(e, "platform_user_id already linked to another user"))?;
        sqlx::query!(
            "UPDATE users SET updated_at = ?1 WHERE id = ?2",
            Utc::now(),
            user_id.get()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(UserPlatform::new(
            UserPlatformId::new(link.id as u32),
            user_id,
            platform_id,
            platform_user_id.to_string(),
            platform_username.to_string(),
        ))
    }

    async fn update_display_name(
        &self,
        user_id: UserId,
        display_name: &str,
    ) -> Result<Option<User>, RepositoryError> {
        let row = sqlx::query!(
            r#"UPDATE users SET display_name = ?1, updated_at = ?2 WHERE id = ?3
               RETURNING id AS "id!: i64", display_name,
                         created_at AS "created_at: DateTime<Utc>",
                         updated_at AS "updated_at: DateTime<Utc>""#,
            display_name,
            Utc::now(),
            user_id.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|row| {
            User::new(
                UserId::new(row.id as u32),
                row.display_name,
                row.created_at,
                row.updated_at,
            )
        }))
    }

    async fn update_platform_username(
        &self,
        user_id: UserId,
        platform_id: PlatformId,
        platform_username: &str,
    ) -> Result<Option<UserPlatform>, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let link = sqlx::query!(
            r#"UPDATE user_platforms SET platform_username = ?1
               WHERE user_id = ?2 AND platform_id = ?3
               RETURNING id AS "id!: i64", user_id AS "user_id!: i64",
                         platform_id AS "platform_id!: i64", platform_user_id"#,
            platform_username,
            user_id.get(),
            platform_id.get()
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_err)?;
        let Some(link) = link else {
            return Ok(None);
        };
        sqlx::query!(
            "UPDATE users SET updated_at = ?1 WHERE id = ?2",
            Utc::now(),
            user_id.get()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(Some(UserPlatform::new(
            UserPlatformId::new(link.id as u32),
            UserId::new(link.user_id as u32),
            PlatformId::new(link.platform_id as u32),
            link.platform_user_id,
            platform_username.to_string(),
        )))
    }

    async fn delete_platform(
        &self,
        user_id: UserId,
        platform_id: PlatformId,
    ) -> Result<bool, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(map_err)?;
        let result = sqlx::query!(
            "DELETE FROM user_platforms WHERE user_id = ? AND platform_id = ?",
            user_id.get(),
            platform_id.get()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        if result.rows_affected() == 0 {
            return Ok(false);
        }
        sqlx::query!(
            "UPDATE users SET updated_at = ?1 WHERE id = ?2",
            Utc::now(),
            user_id.get()
        )
        .execute(&mut *tx)
        .await
        .map_err(map_err)?;
        tx.commit().await.map_err(map_err)?;
        Ok(true)
    }

    async fn delete_user(&self, id: UserId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!("DELETE FROM users WHERE id = ?", id.get())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::Utc;
    use tokio::time::sleep;

    use crate::db::sqlite::test_pool;
    use crate::platform::PlatformId;

    use super::*;

    const TWITCH: PlatformId = PlatformId::TWITCH;
    const YOUTUBE: PlatformId = PlatformId::YOUTUBE;

    async fn repo() -> SqliteUserRepository {
        let (pool, _path) = test_pool().await;
        SqliteUserRepository::new(pool)
    }

    #[tokio::test]
    async fn create_assigns_sequential_ids() {
        let repo = repo().await;
        let first = repo.create("Viewer").await.unwrap();
        let second = repo.create("Another").await.unwrap();

        assert_eq!(first.id.get(), 1);
        assert_eq!(second.id.get(), 2);
        assert_eq!(first.created_at, first.updated_at);
        assert!(repo.get_by_id(first.id).await.unwrap().is_some());
        assert!(repo.get_by_id(UserId::new(999)).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn find_by_platform_joins_linked_user() {
        let repo = repo().await;
        let user = repo.create("Viewer").await.unwrap();

        assert!(
            repo.find_by_platform(TWITCH, "t-1")
                .await
                .unwrap()
                .is_none()
        );

        repo.link_platform(user.id, TWITCH, "t-1", "viewer_ttv")
            .await
            .unwrap();

        let found = repo.find_by_platform(TWITCH, "t-1").await.unwrap().unwrap();
        assert_eq!(found.id, user.id);
        assert_eq!(found.display_name, "Viewer");

        assert!(
            repo.find_by_platform(YOUTUBE, "t-1")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn link_platform_conflict_on_duplicate_across_users() {
        let repo = repo().await;
        let first = repo.create("A").await.unwrap();
        let second = repo.create("B").await.unwrap();

        repo.link_platform(first.id, TWITCH, "same", "a")
            .await
            .unwrap();

        let err = repo
            .link_platform(second.id, TWITCH, "same", "b")
            .await
            .unwrap_err();
        match err {
            RepositoryError::Conflict(m) => {
                assert_eq!(m, "platform_user_id already linked to another user")
            }
            other => panic!("expected Conflict, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn link_platform_bumps_user_updated_at() {
        let repo = repo().await;
        let user = repo.create("Viewer").await.unwrap();
        sleep(Duration::from_millis(5)).await;

        repo.link_platform(user.id, TWITCH, "t-1", "viewer_ttv")
            .await
            .unwrap();

        let after = repo.get_by_id(user.id).await.unwrap().unwrap();
        assert!(after.updated_at > user.updated_at);
    }

    #[tokio::test]
    async fn get_platforms_lists_links_in_order() {
        let repo = repo().await;
        let user = repo.create("Viewer").await.unwrap();

        assert!(repo.get_platforms(user.id).await.unwrap().is_empty());

        repo.link_platform(user.id, YOUTUBE, "y-1", "viewer_yt")
            .await
            .unwrap();
        repo.link_platform(user.id, TWITCH, "t-1", "viewer_ttv")
            .await
            .unwrap();

        let platforms = repo.get_platforms(user.id).await.unwrap();
        assert_eq!(platforms.len(), 2);
        assert_eq!(platforms[0].platform_id, YOUTUBE);
        assert_eq!(platforms[0].platform_user_id, "y-1");
        assert_eq!(platforms[1].platform_id, TWITCH);
    }

    #[tokio::test]
    async fn update_display_name_bumps_and_missing_is_none() {
        let repo = repo().await;
        let user = repo.create("Old").await.unwrap();
        sleep(Duration::from_millis(5)).await;

        let updated = repo
            .update_display_name(user.id, "New")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.display_name, "New");
        assert!(updated.updated_at > user.updated_at);

        assert!(
            repo.update_display_name(UserId::new(999), "X")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn update_platform_username_updates_link_and_bumps_user() {
        let repo = repo().await;
        let user = repo.create("Viewer").await.unwrap();

        assert!(
            repo.update_platform_username(user.id, TWITCH, "new_nick")
                .await
                .unwrap()
                .is_none()
        );

        repo.link_platform(user.id, TWITCH, "t-1", "old_nick")
            .await
            .unwrap();
        sleep(Duration::from_millis(5)).await;

        let link = repo
            .update_platform_username(user.id, TWITCH, "new_nick")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(link.platform_username, "new_nick");
        assert_eq!(link.platform_user_id, "t-1");

        let platforms = repo.get_platforms(user.id).await.unwrap();
        assert_eq!(platforms[0].platform_username, "new_nick");
        let after = repo.get_by_id(user.id).await.unwrap().unwrap();
        assert!(after.updated_at > Utc::now() - chrono::Duration::seconds(1));
    }

    #[tokio::test]
    async fn delete_platform_reports_presence_and_bumps_user() {
        let repo = repo().await;
        let user = repo.create("Viewer").await.unwrap();

        assert!(!repo.delete_platform(user.id, TWITCH).await.unwrap());

        repo.link_platform(user.id, TWITCH, "t-1", "viewer_ttv")
            .await
            .unwrap();
        sleep(Duration::from_millis(5)).await;

        assert!(repo.delete_platform(user.id, TWITCH).await.unwrap());
        assert!(repo.get_platforms(user.id).await.unwrap().is_empty());
        let after = repo.get_by_id(user.id).await.unwrap().unwrap();
        assert!(after.updated_at > user.updated_at);
    }

    #[tokio::test]
    async fn delete_user_cascades_links() {
        let repo = repo().await;
        let user = repo.create("Viewer").await.unwrap();
        repo.link_platform(user.id, TWITCH, "t-1", "viewer_ttv")
            .await
            .unwrap();

        assert!(repo.delete_user(user.id).await.unwrap());
        assert!(!repo.delete_user(user.id).await.unwrap());
        assert!(
            repo.find_by_platform(TWITCH, "t-1")
                .await
                .unwrap()
                .is_none()
        );
        assert!(repo.get_platforms(user.id).await.unwrap().is_empty());
    }
}
