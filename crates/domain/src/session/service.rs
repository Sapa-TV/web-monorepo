use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::admin::service::AdminService;
use crate::config::settings::SharedSettings;
use crate::error::SessionServiceError;
use crate::random::generate_secret;
use crate::session::exchange::{ExchangedToken, LoginExchange};
use model::admin::repository::AdminRepository;
use model::consts::session::LOGIN_TICKET_TTL;
use model::platform::PlatformId;
use model::session::repository::SessionRepository;
use model::session::{LoginTicket, LoginTicketToken, Session, SessionToken};
use model::user::UserId;

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

    pub async fn exchange_login<L: LoginExchange>(
        &self,
        login_exchange: &L,
        code: &str,
        auth_state: &str,
    ) -> Result<(ExchangedToken, LoginTicket), SessionServiceError> {
        let exchanged = login_exchange
            .complete_login(code, auth_state)
            .await
            .map_err(|e| SessionServiceError::Exchange(e.to_string()))?;
        let ticket = self
            .create_login_ticket(
                exchanged.platform_id,
                &exchanged.user_id,
                exchanged.user_name.as_deref(),
            )
            .await?;
        Ok((exchanged, ticket))
    }

    pub async fn create_login_ticket(
        &self,
        platform_id: PlatformId,
        platform_user_id: &str,
        platform_username: Option<&str>,
    ) -> Result<LoginTicket, SessionServiceError> {
        let now = Utc::now();
        let ticket = LoginTicket::new(
            LoginTicketToken::new(generate_secret()),
            platform_id,
            platform_user_id.to_string(),
            platform_username.map(str::to_string),
            now,
            now + LOGIN_TICKET_TTL,
        );
        self.repo.save_ticket(&ticket).await?;
        Ok(ticket)
    }

    pub async fn peek_login_ticket(
        &self,
        ticket: &str,
    ) -> Result<LoginTicket, SessionServiceError> {
        let Some(ticket) = self
            .repo
            .get_ticket(&LoginTicketToken::new(ticket.to_string()))
            .await?
        else {
            return Err(SessionServiceError::InvalidTicket);
        };
        if Utc::now() > ticket.expires_at {
            return Err(SessionServiceError::InvalidTicket);
        }
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

    pub fn ttl(&self) -> Duration {
        Duration::from_secs(self.settings.read().session.ttl_secs)
    }

    pub async fn issue_session_for(
        &self,
        user_id: UserId,
        platform_id: Option<PlatformId>,
        platform_user_id: Option<&str>,
        platform_username: Option<&str>,
        ttl: Duration,
    ) -> Result<Session, SessionServiceError> {
        let now = Utc::now();
        let session = Session::new(
            SessionToken::new(generate_secret()),
            user_id,
            platform_id,
            platform_user_id.map(str::to_string),
            platform_username.map(str::to_string),
            now,
            now + ttl,
        );
        self.repo.save_session(&session).await?;
        Ok(session)
    }

    pub async fn save_session(&self, session: &Session) -> Result<(), SessionServiceError> {
        self.repo.save_session(session).await?;
        Ok(())
    }

    pub async fn reassign_user(
        &self,
        from: UserId,
        to: UserId,
    ) -> Result<usize, SessionServiceError> {
        Ok(self.repo.reassign_user(from, to).await?)
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

    pub async fn prune_expired(&self) -> Result<usize, SessionServiceError> {
        let now = Utc::now();
        let sessions = self.repo.purge_expired_sessions(now).await?;
        let tickets = self.repo.purge_expired_tickets(now).await?;
        Ok(sessions + tickets)
    }
}

#[cfg(test)]
#[path = "service.test.rs"]
mod tests;
