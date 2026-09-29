use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::db::sqlite::map_err;
use crate::error::RepositoryError;
use crate::session::repository::SessionRepository;
use crate::session::{LoginTicket, LoginTicketToken, Session, SessionToken};

#[non_exhaustive]
pub struct SqliteSessionRepository {
    pool: SqlitePool,
}

impl SqliteSessionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl SessionRepository for SqliteSessionRepository {
    async fn save_session(&self, session: &Session) -> Result<(), RepositoryError> {
        sqlx::query!(
            "INSERT INTO sessions (token, twitch_user_id, twitch_user_name, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(token) DO UPDATE SET
                twitch_user_id = excluded.twitch_user_id,
                twitch_user_name = excluded.twitch_user_name,
                created_at = excluded.created_at,
                expires_at = excluded.expires_at",
            session.token.as_str(),
            session.twitch_user_id,
            session.twitch_user_name,
            session.created_at,
            session.expires_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn get_session(&self, token: &SessionToken) -> Result<Option<Session>, RepositoryError> {
        let row = sqlx::query!(
            r#"SELECT token, twitch_user_id, twitch_user_name,
                      created_at AS "created_at: DateTime<Utc>",
                      expires_at AS "expires_at: DateTime<Utc>"
               FROM sessions WHERE token = ?"#,
            token.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|row| {
            Session::new(
                SessionToken::new(row.token),
                row.twitch_user_id,
                row.twitch_user_name,
                row.created_at,
                row.expires_at,
            )
        }))
    }

    async fn delete_session(&self, token: &SessionToken) -> Result<bool, RepositoryError> {
        let result = sqlx::query!("DELETE FROM sessions WHERE token = ?", token.as_str())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn purge_expired_sessions(&self, now: DateTime<Utc>) -> Result<usize, RepositoryError> {
        let result = sqlx::query!("DELETE FROM sessions WHERE expires_at <= ?", now)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() as usize)
    }

    async fn save_ticket(&self, ticket: &LoginTicket) -> Result<(), RepositoryError> {
        sqlx::query!(
            "INSERT INTO login_tickets (ticket, twitch_user_id, twitch_user_name, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(ticket) DO UPDATE SET
                twitch_user_id = excluded.twitch_user_id,
                twitch_user_name = excluded.twitch_user_name,
                created_at = excluded.created_at,
                expires_at = excluded.expires_at",
            ticket.ticket.as_str(),
            ticket.twitch_user_id,
            ticket.twitch_user_name,
            ticket.created_at,
            ticket.expires_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn take_ticket(
        &self,
        ticket: &LoginTicketToken,
    ) -> Result<Option<LoginTicket>, RepositoryError> {
        let row = sqlx::query!(
            r#"DELETE FROM login_tickets WHERE ticket = ?
               RETURNING ticket, twitch_user_id, twitch_user_name,
                         created_at AS "created_at: DateTime<Utc>",
                         expires_at AS "expires_at: DateTime<Utc>""#,
            ticket.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|row| {
            LoginTicket::new(
                LoginTicketToken::new(row.ticket),
                row.twitch_user_id,
                row.twitch_user_name,
                row.created_at,
                row.expires_at,
            )
        }))
    }

    async fn purge_expired_tickets(&self, now: DateTime<Utc>) -> Result<usize, RepositoryError> {
        let result = sqlx::query!("DELETE FROM login_tickets WHERE expires_at <= ?", now)
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() as usize)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{Duration as ChronoDuration, Utc};

    use crate::db::sqlite::test_pool;

    use super::*;

    async fn repo() -> SqliteSessionRepository {
        let (pool, _path) = test_pool().await;
        SqliteSessionRepository::new(pool)
    }

    fn sample_session(token: &str) -> Session {
        let now = Utc::now();
        Session::new(
            SessionToken::new(token),
            "123".to_string(),
            Some("tester".to_string()),
            now,
            now + Duration::from_secs(3600),
        )
    }

    #[tokio::test]
    async fn session_roundtrip_and_double_delete() {
        let repo = repo().await;
        let session = sample_session("tok");

        repo.save_session(&session).await.unwrap();

        let fetched = repo.get_session(&session.token).await.unwrap().unwrap();
        assert_eq!(fetched.twitch_user_id, "123");
        assert_eq!(fetched.twitch_user_name.as_deref(), Some("tester"));

        assert!(repo.delete_session(&session.token).await.unwrap());
        assert!(!repo.delete_session(&session.token).await.unwrap());
        assert!(repo.get_session(&session.token).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn save_session_upserts_same_token() {
        let repo = repo().await;
        let mut session = sample_session("tok");

        repo.save_session(&session).await.unwrap();
        session.twitch_user_id = "456".to_string();
        repo.save_session(&session).await.unwrap();

        let fetched = repo.get_session(&session.token).await.unwrap().unwrap();
        assert_eq!(fetched.twitch_user_id, "456");
    }

    #[tokio::test]
    async fn ticket_take_is_destructive() {
        let repo = repo().await;
        let now = Utc::now();
        let ticket = LoginTicket::new(
            LoginTicketToken::new("tic"),
            "123".to_string(),
            None,
            now,
            now + ChronoDuration::seconds(600),
        );

        repo.save_ticket(&ticket).await.unwrap();

        let taken = repo.take_ticket(&ticket.ticket).await.unwrap().unwrap();
        assert_eq!(taken.twitch_user_id, "123");
        assert!(taken.twitch_user_name.is_none());

        assert!(repo.take_ticket(&ticket.ticket).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn purge_removes_only_expired_boundary_inclusive() {
        let repo = repo().await;
        let now = Utc::now();

        for (token, ttl) in [
            ("expired", ChronoDuration::hours(-1)),
            ("boundary", ChronoDuration::zero()),
            ("fresh", ChronoDuration::hours(1)),
        ] {
            let session = Session::new(
                SessionToken::new(token),
                "1".to_string(),
                None,
                now,
                now + ttl,
            );
            repo.save_session(&session).await.unwrap();
        }

        let removed = repo.purge_expired_sessions(now).await.unwrap();

        assert_eq!(removed, 2);
        assert!(
            repo.get_session(&SessionToken::new("fresh"))
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            repo.get_session(&SessionToken::new("expired"))
                .await
                .unwrap()
                .is_none()
        );

        let removed_tickets = repo.purge_expired_tickets(now).await.unwrap();
        assert_eq!(removed_tickets, 0);
    }
}
