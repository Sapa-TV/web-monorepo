use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::admin::{Admin, repository::AdminRepository};
use crate::db::sqlite::{conflict_on_unique, map_err};
use crate::error::RepositoryError;

#[non_exhaustive]
pub struct SqliteAdminRepository {
    pool: SqlitePool,
}

impl SqliteAdminRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl AdminRepository for SqliteAdminRepository {
    async fn create(
        &self,
        twitch_id: &str,
        display_name: Option<&str>,
        is_root: bool,
    ) -> Result<Admin, RepositoryError> {
        let created_at = Utc::now();
        sqlx::query!(
            "INSERT INTO admins (twitch_id, display_name, is_root, created_at) VALUES (?, ?, ?, ?)",
            twitch_id,
            display_name,
            is_root,
            created_at
        )
        .execute(&self.pool)
        .await
        .map_err(|e| conflict_on_unique(e, "admin with this twitch_id already exists"))?;
        Ok(Admin {
            twitch_id: twitch_id.to_string(),
            display_name: display_name.map(str::to_string),
            is_root,
            created_at,
        })
    }

    async fn get_by_twitch_id(&self, twitch_id: &str) -> Result<Option<Admin>, RepositoryError> {
        let admin = sqlx::query_as!(
            Admin,
            r#"SELECT twitch_id, display_name, is_root AS "is_root: bool", created_at AS "created_at: DateTime<Utc>" FROM admins WHERE twitch_id = ?"#,
            twitch_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(admin)
    }

    async fn list(&self) -> Result<Vec<Admin>, RepositoryError> {
        let admins = sqlx::query_as!(
            Admin,
            r#"SELECT twitch_id, display_name, is_root AS "is_root: bool", created_at AS "created_at: DateTime<Utc>" FROM admins ORDER BY rowid"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(admins)
    }

    async fn update_display_name(
        &self,
        twitch_id: &str,
        display_name: &str,
    ) -> Result<Option<Admin>, RepositoryError> {
        let admin = sqlx::query_as!(
            Admin,
            r#"UPDATE admins SET display_name = ?1 WHERE twitch_id = ?2
               RETURNING twitch_id, display_name, is_root AS "is_root: bool", created_at AS "created_at: DateTime<Utc>""#,
            display_name,
            twitch_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(admin)
    }

    async fn set_root(
        &self,
        twitch_id: &str,
        is_root: bool,
    ) -> Result<Option<Admin>, RepositoryError> {
        let admin = sqlx::query_as!(
            Admin,
            r#"UPDATE admins SET is_root = ?1 WHERE twitch_id = ?2
               RETURNING twitch_id, display_name, is_root AS "is_root: bool", created_at AS "created_at: DateTime<Utc>""#,
            is_root,
            twitch_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(admin)
    }

    async fn delete_by_twitch_id(&self, twitch_id: &str) -> Result<bool, RepositoryError> {
        let result = sqlx::query!("DELETE FROM admins WHERE twitch_id = ?", twitch_id)
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

    async fn repo() -> SqliteAdminRepository {
        let (pool, _path) = test_pool().await;
        SqliteAdminRepository::new(pool)
    }

    #[tokio::test]
    async fn create_and_get() {
        let repo = repo().await;
        let admin = repo.create("100", Some("sapushka_"), true).await.unwrap();
        assert!(admin.is_root);

        let fetched = repo.get_by_twitch_id("100").await.unwrap().unwrap();
        assert_eq!(fetched.twitch_id, "100");
        assert_eq!(fetched.display_name.as_deref(), Some("sapushka_"));
    }

    #[tokio::test]
    async fn create_conflict_on_duplicate() {
        let repo = repo().await;
        repo.create("100", None, false).await.unwrap();

        let err = repo.create("100", None, true).await.unwrap_err();
        assert!(matches!(
            err,
            RepositoryError::Conflict(ref m) if m.contains("already exists")
        ));
    }

    #[tokio::test]
    async fn update_display_name_missing_is_none() {
        let repo = repo().await;
        let updated = repo.update_display_name("404", "nobody").await.unwrap();
        assert!(updated.is_none());
    }

    #[tokio::test]
    async fn update_display_name_and_list_order() {
        let repo = repo().await;
        repo.create("2", None, false).await.unwrap();
        repo.create("1", None, false).await.unwrap();

        let updated = repo.update_display_name("2", "second").await.unwrap().unwrap();
        assert_eq!(updated.display_name.as_deref(), Some("second"));

        let listed = repo.list().await.unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].twitch_id, "2");
        assert_eq!(listed[1].twitch_id, "1");
    }

    #[tokio::test]
    async fn set_root_updates_flag() {
        let repo = repo().await;
        repo.create("7", None, false).await.unwrap();

        let promoted = repo.set_root("7", true).await.unwrap().unwrap();
        assert!(promoted.is_root);
        assert!(!repo.set_root("404", true).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn delete_reports_presence() {
        let repo = repo().await;
        repo.create("9", None, false).await.unwrap();

        assert!(repo.delete_by_twitch_id("9").await.unwrap());
        assert!(!repo.delete_by_twitch_id("9").await.unwrap());
    }
}
