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
        Ok(Admin::new(
            twitch_id.to_string(),
            display_name.map(str::to_string),
            is_root,
            created_at,
        ))
    }

    async fn get_by_twitch_id(&self, twitch_id: &str) -> Result<Option<Admin>, RepositoryError> {
        let row = sqlx::query!(
            r#"SELECT twitch_id, display_name, is_root AS "is_root: bool", created_at AS "created_at: DateTime<Utc>" FROM admins WHERE twitch_id = ?"#,
            twitch_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| Admin::new(r.twitch_id, r.display_name, r.is_root, r.created_at)))
    }

    async fn list(&self) -> Result<Vec<Admin>, RepositoryError> {
        let rows = sqlx::query!(
            r#"SELECT twitch_id, display_name, is_root AS "is_root: bool", created_at AS "created_at: DateTime<Utc>" FROM admins ORDER BY rowid"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|r| Admin::new(r.twitch_id, r.display_name, r.is_root, r.created_at))
            .collect())
    }

    async fn update_display_name(
        &self,
        twitch_id: &str,
        display_name: &str,
    ) -> Result<Option<Admin>, RepositoryError> {
        let row = sqlx::query!(
            r#"UPDATE admins SET display_name = ?1 WHERE twitch_id = ?2
               RETURNING twitch_id, display_name, is_root AS "is_root: bool", created_at AS "created_at: DateTime<Utc>""#,
            display_name,
            twitch_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| Admin::new(r.twitch_id, r.display_name, r.is_root, r.created_at)))
    }

    async fn set_root(
        &self,
        twitch_id: &str,
        is_root: bool,
    ) -> Result<Option<Admin>, RepositoryError> {
        let row = sqlx::query!(
            r#"UPDATE admins SET is_root = ?1 WHERE twitch_id = ?2
               RETURNING twitch_id, display_name, is_root AS "is_root: bool", created_at AS "created_at: DateTime<Utc>""#,
            is_root,
            twitch_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|r| Admin::new(r.twitch_id, r.display_name, r.is_root, r.created_at)))
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
#[path = "admin.test.rs"]
mod tests;
