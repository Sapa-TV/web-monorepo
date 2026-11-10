use std::sync::Arc;
use std::time::Duration;

use crate::admin::service::AdminService;
use crate::error::SessionServiceError;
use crate::session::service::SessionService;
use crate::user::service::UserService;
use model::admin::repository::AdminRepository;
use model::error::RepositoryError;
use model::platform::PlatformRepository;
use model::session::Session;
use model::session::repository::SessionRepository;
use model::user::repository::UserRepository;

#[non_exhaustive]
pub struct AccountService<R, A, U, P>
where
    R: SessionRepository,
    A: AdminRepository,
    U: UserRepository,
    P: PlatformRepository,
{
    sessions: Arc<SessionService<R, A>>,
    users: Arc<UserService<U, P>>,
    guest_ttl: Duration,
}

impl<R, A, U, P> AccountService<R, A, U, P>
where
    R: SessionRepository,
    A: AdminRepository,
    U: UserRepository,
    P: PlatformRepository,
{
    pub fn new(
        sessions: Arc<SessionService<R, A>>,
        users: Arc<UserService<U, P>>,
        guest_ttl: Duration,
    ) -> Self {
        Self {
            sessions,
            users,
            guest_ttl,
        }
    }

    pub fn sessions(&self) -> &SessionService<R, A> {
        &self.sessions
    }

    pub fn admin(&self) -> &AdminService<A> {
        self.sessions.admin()
    }

    pub async fn guest(
        &self,
        current_token: Option<&str>,
    ) -> Result<(Session, bool), SessionServiceError> {
        if let Some(token) = current_token
            && let Ok(session) = self.sessions.validate_session(token).await
        {
            return Ok((session, false));
        }
        let user = self.users.create("guest").await?;
        let session = self
            .sessions
            .issue_session_for(user.id, None, None, self.guest_ttl)
            .await?;
        Ok((session, false))
    }

    pub async fn login(
        &self,
        current_token: Option<&str>,
        login_cookie: Option<&str>,
        ticket: &str,
    ) -> Result<(Session, bool), SessionServiceError> {
        if login_cookie != Some(ticket) {
            return Err(SessionServiceError::InvalidTicket);
        }
        let ticket = self.sessions.consume_login_ticket(ticket).await?;
        let twitch_id = ticket.twitch_user_id.as_str();
        let twitch_name = ticket.twitch_user_name.as_deref();
        let display_name = twitch_name.unwrap_or(twitch_id);

        let current = match current_token {
            Some(token) => self.sessions.validate_session(token).await.ok(),
            None => None,
        };
        let existing = self.users.find_by_platform("twitch", twitch_id).await?;

        let target = match existing {
            Some(user) => user.id,
            None => {
                let user_id = match current.as_ref() {
                    Some(session) => session.user_id,
                    None => self.users.create(display_name).await?.id,
                };
                self.users
                    .link_platform(user_id, "twitch", twitch_id, display_name)
                    .await?;
                user_id
            }
        };

        if let Some(user) = self.users.get_user(target).await?
            && user.display_name != display_name
        {
            self.users.update_user(target, display_name).await.ok();
        }

        let session = match current {
            Some(mut session) if session.user_id != target => {
                let platforms = self.users.get_platforms(session.user_id).await?;
                if !platforms.is_empty() {
                    return Err(SessionServiceError::AccountConflict);
                }
                self.sessions.reassign_user(session.user_id, target).await?;
                self.users.merge_users(session.user_id, target).await?;
                session.user_id = target;
                session.twitch_user_id = Some(twitch_id.to_string());
                session.twitch_user_name = twitch_name.map(str::to_string);
                self.sessions.save_session(&session).await?;
                session
            }
            Some(mut session) => {
                session.user_id = target;
                session.twitch_user_id = Some(twitch_id.to_string());
                session.twitch_user_name = twitch_name.map(str::to_string);
                self.sessions.save_session(&session).await?;
                session
            }
            None => {
                self.sessions
                    .issue_session_for(target, Some(twitch_id), twitch_name, self.sessions.ttl())
                    .await?
            }
        };

        let is_admin = self.admin().is_admin(twitch_id).await.map_err(|_| {
            SessionServiceError::Repo(RepositoryError::Database("admin service".into()))
        })?;
        if is_admin {
            self.admin()
                .update_display_name(twitch_id, display_name)
                .await
                .ok();
        }
        let is_root = self.admin().is_root(twitch_id).await.map_err(|_| {
            SessionServiceError::Repo(RepositoryError::Database("admin service".into()))
        })?;
        Ok((session, is_root))
    }
}

#[cfg(test)]
#[path = "account.test.rs"]
mod tests;
