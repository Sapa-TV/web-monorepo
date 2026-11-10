use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::map_err;
use model::error::RepositoryError;
use model::platform::PlatformId;
use model::session::repository::SessionRepository;
use model::session::{LoginTicket, LoginTicketToken, Session, SessionToken};
use model::user::UserId;

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
            "INSERT INTO sessions (token, user_id, platform_id, platform_user_id, platform_username, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(token) DO UPDATE SET
                user_id = excluded.user_id,
                platform_id = excluded.platform_id,
                platform_user_id = excluded.platform_user_id,
                platform_username = excluded.platform_username,
                created_at = excluded.created_at,
                expires_at = excluded.expires_at",
            session.token.as_str(),
            session.user_id.get(),
            session.platform_id.map(PlatformId::get),
            session.platform_user_id,
            session.platform_username,
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
            r#"SELECT token, user_id AS "user_id!: i64",
                      platform_id AS "platform_id: i64", platform_user_id, platform_username,
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
                UserId::new(row.user_id as u32),
                row.platform_id.map(|id| PlatformId::new(id as u32)),
                row.platform_user_id,
                row.platform_username,
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

    async fn reassign_user(&self, from: UserId, to: UserId) -> Result<usize, RepositoryError> {
        let result = sqlx::query!(
            "UPDATE sessions SET user_id = ? WHERE user_id = ?",
            to.get(),
            from.get()
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(result.rows_affected() as usize)
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
            "INSERT INTO login_tickets (ticket, platform_id, platform_user_id, platform_username, created_at, expires_at)
             VALUES (?, ?, ?, ?, ?, ?)
             ON CONFLICT(ticket) DO UPDATE SET
                platform_id = excluded.platform_id,
                platform_user_id = excluded.platform_user_id,
                platform_username = excluded.platform_username,
                created_at = excluded.created_at,
                expires_at = excluded.expires_at",
            ticket.ticket.as_str(),
            ticket.platform_id.get(),
            ticket.platform_user_id,
            ticket.platform_username,
            ticket.created_at,
            ticket.expires_at
        )
        .execute(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(())
    }

    async fn get_ticket(
        &self,
        ticket: &LoginTicketToken,
    ) -> Result<Option<LoginTicket>, RepositoryError> {
        let row = sqlx::query!(
            r#"SELECT ticket, platform_id AS "platform_id!: i64", platform_user_id, platform_username,
                      created_at AS "created_at: DateTime<Utc>",
                      expires_at AS "expires_at: DateTime<Utc>"
               FROM login_tickets WHERE ticket = ?"#,
            ticket.as_str()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|row| {
            LoginTicket::new(
                LoginTicketToken::new(row.ticket),
                PlatformId::new(row.platform_id as u32),
                row.platform_user_id,
                row.platform_username,
                row.created_at,
                row.expires_at,
            )
        }))
    }

    async fn take_ticket(
        &self,
        ticket: &LoginTicketToken,
    ) -> Result<Option<LoginTicket>, RepositoryError> {
        let row = sqlx::query!(
            r#"DELETE FROM login_tickets WHERE ticket = ?
               RETURNING ticket, platform_id AS "platform_id!: i64", platform_user_id, platform_username,
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
                PlatformId::new(row.platform_id as u32),
                row.platform_user_id,
                row.platform_username,
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
#[path = "session.test.rs"]
mod tests;
