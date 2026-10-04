use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::admin::auth::{AdminAuthService, ExchangedToken};
use crate::admin::repository::AdminRepository;
use crate::admin::service::AdminService;
use crate::config::store::SharedSettings;
use crate::consts::session::LOGIN_TICKET_TTL;
use crate::error::RepositoryError;
use crate::error::SessionServiceError;
use crate::platform::PlatformCredentialRepository;
use crate::session::repository::SessionRepository;
use crate::session::{LoginTicket, LoginTicketToken, Session, SessionToken};

#[non_exhaustive]
pub struct SessionService<R, A>
where
    R: SessionRepository,
    A: AdminRepository,
{
    repo: Arc<R>,
    admin: Arc<AdminService<A>>,
    settings: SharedSettings,
}

impl<R, A> SessionService<R, A>
where
    R: SessionRepository,
    A: AdminRepository,
{
    pub fn new(repo: Arc<R>, admin: Arc<AdminService<A>>, settings: SharedSettings) -> Self {
        Self {
            repo,
            admin,
            settings,
        }
    }

    pub fn admin(&self) -> &AdminService<A> {
        &self.admin
    }

    pub async fn exchange_login<C>(
        &self,
        admin_auth: &AdminAuthService<C>,
        code: &str,
        auth_state: &str,
    ) -> Result<(ExchangedToken, LoginTicket), SessionServiceError>
    where
        C: PlatformCredentialRepository,
    {
        let exchanged = admin_auth
            .complete_login(code, auth_state)
            .await
            .map_err(|e| SessionServiceError::Exchange(e.to_string()))?;
        let ticket = self
            .create_login_ticket(&exchanged.user_id, exchanged.user_name.as_deref())
            .await?;
        Ok((exchanged, ticket))
    }

    pub async fn create_login_ticket(
        &self,
        twitch_user_id: &str,
        twitch_user_name: Option<&str>,
    ) -> Result<LoginTicket, SessionServiceError> {
        let now = Utc::now();
        let ticket = LoginTicket::new(
            LoginTicketToken::new(nonce()),
            twitch_user_id.to_string(),
            twitch_user_name.map(str::to_string),
            now,
            now + LOGIN_TICKET_TTL,
        );
        self.repo.save_ticket(&ticket).await?;
        Ok(ticket)
    }

    pub async fn consume_login_ticket(
        &self,
        ticket: &str,
    ) -> Result<LoginTicket, SessionServiceError> {
        let Some(ticket) = self
            .repo
            .take_ticket(&LoginTicketToken::new(ticket.to_string()))
            .await?
        else {
            return Err(SessionServiceError::InvalidTicket);
        };
        if Utc::now() > ticket.expires_at {
            return Err(SessionServiceError::InvalidTicket);
        }
        Ok(ticket)
    }

    pub async fn issue_session(
        &self,
        twitch_user_id: &str,
        twitch_user_name: Option<&str>,
    ) -> Result<Session, SessionServiceError> {
        let now = Utc::now();
        let ttl = Duration::from_secs(self.settings.read().session.ttl_secs);
        let session = Session::new(
            SessionToken::new(nonce()),
            twitch_user_id.to_string(),
            twitch_user_name.map(str::to_string),
            now,
            now + ttl,
        );
        self.repo.save_session(&session).await?;
        Ok(session)
    }

    pub async fn validate_session(&self, token: &str) -> Result<Session, SessionServiceError> {
        let Some(session) = self
            .repo
            .get_session(&SessionToken::new(token.to_string()))
            .await?
        else {
            return Err(SessionServiceError::SessionNotFound);
        };
        if Utc::now() > session.expires_at {
            return Err(SessionServiceError::SessionExpired);
        }
        Ok(session)
    }

    pub async fn logout(&self, token: &str) -> Result<(), SessionServiceError> {
        self.repo
            .delete_session(&SessionToken::new(token.to_string()))
            .await?;
        Ok(())
    }

    pub async fn login(
        &self,
        login_cookie: Option<&str>,
        ticket: &str,
    ) -> Result<(Session, bool), SessionServiceError> {
        if login_cookie != Some(ticket) {
            return Err(SessionServiceError::InvalidTicket);
        }

        let ticket = self.consume_login_ticket(ticket).await?;

        let is_admin = self
            .admin
            .is_admin(&ticket.twitch_user_id)
            .await
            .map_err(|_| {
                SessionServiceError::Repo(RepositoryError::Database("admin service".to_string()))
            })?;
        tracing::debug!(
            "login: twitch_user_id={}, is_admin={}",
            ticket.twitch_user_id,
            is_admin
        );
        if is_admin {
            self.admin
                .update_display_name(
                    &ticket.twitch_user_id,
                    ticket.twitch_user_name.as_deref().unwrap_or(""),
                )
                .await
                .ok();
        }

        let session = self
            .issue_session(&ticket.twitch_user_id, ticket.twitch_user_name.as_deref())
            .await?;

        let is_root = self
            .admin
            .is_root(&session.twitch_user_id)
            .await
            .map_err(|_| {
                SessionServiceError::Repo(RepositoryError::Database("admin service".to_string()))
            })?;

        tracing::debug!(
            "session issued: twitch_user_id={}, is_root={}",
            session.twitch_user_id,
            is_root
        );

        Ok((session, is_root))
    }

    pub async fn prune_expired(&self) -> Result<usize, SessionServiceError> {
        let now = Utc::now();
        let sessions = self.repo.purge_expired_sessions(now).await?;
        let tickets = self.repo.purge_expired_tickets(now).await?;
        Ok(sessions + tickets)
    }
}

fn nonce() -> String {
    use rand::RngExt;
    let hi: u128 = rand::rng().random();
    let lo: u128 = rand::rng().random();
    format!("{hi:032x}{lo:032x}")
}

#[cfg(test)]
#[path = "service.test.rs"]
mod tests;
