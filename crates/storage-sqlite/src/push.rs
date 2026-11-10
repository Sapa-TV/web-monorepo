use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::map_err;
use model::error::RepositoryError;
use model::push::repository::PushSubscriptionRepository;
use model::push::{NewPushSubscription, PushSubscription, PushSubscriptionId};
use model::user::UserId;

#[non_exhaustive]
pub struct SqlitePushSubscriptionRepository {
    pool: SqlitePool,
}

impl SqlitePushSubscriptionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl PushSubscriptionRepository for SqlitePushSubscriptionRepository {
    async fn upsert(
        &self,
        subscription: &NewPushSubscription,
    ) -> Result<PushSubscription, RepositoryError> {
        let now = Utc::now();
        let row = sqlx::query!(
            r#"INSERT INTO push_subscriptions
                   (user_id, endpoint, p256dh, auth, user_agent, created_at, last_seen_at, failure_count)
               VALUES (?, ?, ?, ?, ?, ?, ?, 0)
               ON CONFLICT(endpoint) DO UPDATE SET
                   user_id = excluded.user_id,
                   p256dh = excluded.p256dh,
                   auth = excluded.auth,
                   user_agent = excluded.user_agent,
                   last_seen_at = excluded.last_seen_at,
                   failure_count = 0
               RETURNING id AS "id!: i64", user_id AS "user_id!: i64", endpoint, p256dh, auth,
                         user_agent, created_at AS "created_at: DateTime<Utc>",
                         last_seen_at AS "last_seen_at: DateTime<Utc>",
                         failure_count AS "failure_count!: i64""#,
            subscription.user_id.get(),
            subscription.endpoint,
            subscription.p256dh,
            subscription.auth,
            subscription.user_agent,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(map_row(
            row.id,
            row.user_id,
            row.endpoint,
            row.p256dh,
            row.auth,
            row.user_agent,
            row.created_at,
            row.last_seen_at,
            row.failure_count,
        ))
    }

    async fn list_all(&self) -> Result<Vec<PushSubscription>, RepositoryError> {
        let rows = sqlx::query!(
            r#"SELECT id AS "id!: i64", user_id AS "user_id!: i64", endpoint, p256dh, auth,
                      user_agent, created_at AS "created_at: DateTime<Utc>",
                      last_seen_at AS "last_seen_at: DateTime<Utc>",
                      failure_count AS "failure_count!: i64"
               FROM push_subscriptions ORDER BY id"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|row| {
                map_row(
                    row.id,
                    row.user_id,
                    row.endpoint,
                    row.p256dh,
                    row.auth,
                    row.user_agent,
                    row.created_at,
                    row.last_seen_at,
                    row.failure_count,
                )
            })
            .collect())
    }

    async fn list_by_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<PushSubscription>, RepositoryError> {
        let rows = sqlx::query!(
            r#"SELECT id AS "id!: i64", user_id AS "user_id!: i64", endpoint, p256dh, auth,
                      user_agent, created_at AS "created_at: DateTime<Utc>",
                      last_seen_at AS "last_seen_at: DateTime<Utc>",
                      failure_count AS "failure_count!: i64"
               FROM push_subscriptions WHERE user_id = ? ORDER BY id"#,
            user_id.get()
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(rows
            .into_iter()
            .map(|row| {
                map_row(
                    row.id,
                    row.user_id,
                    row.endpoint,
                    row.p256dh,
                    row.auth,
                    row.user_agent,
                    row.created_at,
                    row.last_seen_at,
                    row.failure_count,
                )
            })
            .collect())
    }

    async fn delete_by_user_and_endpoint(
        &self,
        user_id: UserId,
        endpoint: &str,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            "DELETE FROM push_subscriptions WHERE user_id = ? AND endpoint = ?",
            user_id.get(),
            endpoint
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn record_delivery(&self, id: PushSubscriptionId) -> Result<(), RepositoryError> {
        sqlx::query!(
            "UPDATE push_subscriptions SET last_seen_at = ?, failure_count = 0 WHERE id = ?",
            Utc::now(),
            id.get()
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn record_failure(&self, id: PushSubscriptionId) -> Result<(), RepositoryError> {
        sqlx::query!(
            "UPDATE push_subscriptions SET failure_count = failure_count + 1 WHERE id = ?",
            id.get()
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn delete(&self, id: PushSubscriptionId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!("DELETE FROM push_subscriptions WHERE id = ?", id.get())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() > 0)
    }
}

#[allow(clippy::too_many_arguments)]
fn map_row(
    id: i64,
    user_id: i64,
    endpoint: String,
    p256dh: String,
    auth: String,
    user_agent: Option<String>,
    created_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
    failure_count: i64,
) -> PushSubscription {
    PushSubscription::new(
        PushSubscriptionId::new(id as u32),
        UserId::new(user_id as u32),
        endpoint,
        p256dh,
        auth,
        user_agent,
        created_at,
        last_seen_at,
        failure_count as u32,
    )
}

#[cfg(test)]
#[path = "push.test.rs"]
mod tests;
