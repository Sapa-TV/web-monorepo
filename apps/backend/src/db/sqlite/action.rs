use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::actions::action::{Action, ActionId, ActionKind};
use crate::actions::repository::ActionRepository;
use crate::db::sqlite::map_err;
use crate::error::RepositoryError;

#[non_exhaustive]
pub struct SqliteActionRepository {
    pool: SqlitePool,
}

impl SqliteActionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn parse_kind(kind: &str) -> Result<ActionKind, RepositoryError> {
    serde_json::from_str(kind)
        .map_err(|e| RepositoryError::Database(format!("invalid action kind: {e}")))
}

fn action_from_row(
    id: i64,
    name: String,
    kind: String,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> Result<Action, RepositoryError> {
    Ok(Action::new(
        ActionId::new(id as u32),
        name,
        parse_kind(&kind)?,
        enabled,
        created_at,
        updated_at,
    ))
}

impl ActionRepository for SqliteActionRepository {
    async fn create(
        &self,
        name: &str,
        kind: ActionKind,
        enabled: bool,
    ) -> Result<Action, RepositoryError> {
        let now = Utc::now();
        let kind_json = serde_json::to_string(&kind)
            .map_err(|e| RepositoryError::Database(format!("serialize action kind failed: {e}")))?;
        let row = sqlx::query!(
            "INSERT INTO actions (name, kind, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?)
             RETURNING id AS \"id!: i64\"",
            name,
            kind_json,
            enabled,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(Action::new(
            ActionId::new(row.id as u32),
            name.to_string(),
            kind,
            enabled,
            now,
            now,
        ))
    }

    async fn get_by_id(&self, id: ActionId) -> Result<Option<Action>, RepositoryError> {
        let row = sqlx::query!(
            r#"SELECT id AS "id!: i64", name, kind,
                      enabled AS "enabled: bool",
                      created_at AS "created_at: DateTime<Utc>",
                      updated_at AS "updated_at: DateTime<Utc>"
               FROM actions WHERE id = ?"#,
            id.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        match row {
            Some(row) => Ok(Some(action_from_row(
                row.id,
                row.name,
                row.kind,
                row.enabled,
                row.created_at,
                row.updated_at,
            )?)),
            None => Ok(None),
        }
    }

    async fn list(&self) -> Result<Vec<Action>, RepositoryError> {
        let rows = sqlx::query!(
            r#"SELECT id AS "id!: i64", name, kind,
                      enabled AS "enabled: bool",
                      created_at AS "created_at: DateTime<Utc>",
                      updated_at AS "updated_at: DateTime<Utc>"
               FROM actions ORDER BY id"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        rows.into_iter()
            .map(|row| {
                action_from_row(
                    row.id,
                    row.name,
                    row.kind,
                    row.enabled,
                    row.created_at,
                    row.updated_at,
                )
            })
            .collect()
    }

    async fn update(&self, action: Action) -> Result<Option<Action>, RepositoryError> {
        let now = Utc::now();
        let kind_json = serde_json::to_string(&action.kind)
            .map_err(|e| RepositoryError::Database(format!("serialize action kind failed: {e}")))?;
        let row = sqlx::query!(
            r#"UPDATE actions SET name = ?1, kind = ?2, enabled = ?3, updated_at = ?4 WHERE id = ?5
               RETURNING created_at AS "created_at: DateTime<Utc>""#,
            action.name,
            kind_json,
            action.enabled,
            now,
            action.id.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|row| {
            Action::new(
                action.id,
                action.name.clone(),
                action.kind.clone(),
                action.enabled,
                row.created_at,
                now,
            )
        }))
    }

    async fn delete(&self, id: ActionId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!("DELETE FROM actions WHERE id = ?", id.get())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::sleep;

    use crate::db::sqlite::test_pool;

    use super::*;

    async fn repo() -> SqliteActionRepository {
        let (pool, _path) = test_pool().await;
        SqliteActionRepository::new(pool)
    }

    #[tokio::test]
    async fn create_and_get_roundtrip_all_kinds() {
        let repo = repo().await;

        for (name, kind) in [
            ("noop", ActionKind::NoAction),
            ("spin", ActionKind::EnqueueRoulette),
            (
                "reply",
                ActionKind::ChatReply {
                    message_template: "hi {username}".to_string(),
                },
            ),
        ] {
            let saved = repo.create(name, kind.clone(), true).await.unwrap();
            let fetched = repo.get_by_id(saved.id).await.unwrap().unwrap();
            assert_eq!(fetched.name, name);
            assert_eq!(fetched.kind, kind);
            assert_eq!(fetched.created_at, fetched.updated_at);
        }

        assert!(repo.get_by_id(ActionId::new(999)).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_returns_in_insertion_order() {
        let repo = repo().await;
        assert!(repo.list().await.unwrap().is_empty());

        repo.create("a", ActionKind::NoAction, true).await.unwrap();
        repo.create("b", ActionKind::EnqueueRoulette, false)
            .await
            .unwrap();

        let all = repo.list().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "a");
        assert_eq!(all[1].name, "b");
        assert!(!all[1].enabled);
    }

    #[tokio::test]
    async fn update_preserves_created_at_and_missing_is_none() {
        let repo = repo().await;
        let saved = repo
            .create("old", ActionKind::NoAction, true)
            .await
            .unwrap();
        sleep(Duration::from_millis(5)).await;

        let mut next = saved.clone();
        next.name = "new".to_string();
        next.kind = ActionKind::ChatReply {
            message_template: "{text}".to_string(),
        };
        next.enabled = false;
        let updated = repo.update(next).await.unwrap().unwrap();

        assert_eq!(updated.name, "new");
        assert_eq!(updated.created_at, saved.created_at);
        assert!(updated.updated_at > saved.updated_at);

        let mut missing = saved.clone();
        missing.id = ActionId::new(999);
        assert!(repo.update(missing).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_reports_presence() {
        let repo = repo().await;
        let saved = repo.create("x", ActionKind::NoAction, true).await.unwrap();

        assert!(repo.delete(saved.id).await.unwrap());
        assert!(!repo.delete(saved.id).await.unwrap());
    }
}
