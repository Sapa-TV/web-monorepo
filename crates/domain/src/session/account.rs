use std::sync::Arc;
use std::time::Duration;

use crate::admin::service::AdminService;
use crate::error::SessionServiceError;
use crate::session::service::SessionService;
use crate::user::service::UserService;
use model::admin::repository::AdminRepository;
use model::error::RepositoryError;
use model::platform::{PlatformId, PlatformRepository};
use model::session::Session;
use model::session::repository::SessionRepository;
use model::user::UserId;
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
            .issue_session_for(user.id, None, None, None, self.guest_ttl)
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
        let platform_id = ticket.platform_id;
        let platform_name = platform_id.name();
        let platform_user_id = ticket.platform_user_id.as_str();
        let platform_username = ticket.platform_username.as_deref();
        let display_name = platform_username.unwrap_or(platform_user_id);

        let current = match current_token {
            Some(token) => self.sessions.validate_session(token).await.ok(),
            None => None,
        };
        let existing = self
            .users
            .find_by_platform(platform_name, platform_user_id)
            .await?;

        let target = match existing {
            Some(user) => user.id,
            None => {
                let user_id = match current.as_ref() {
                    Some(session) => session.user_id,
                    None => self.users.create(display_name).await?.id,
                };
                self.users
                    .link_platform(user_id, platform_name, platform_user_id, display_name)
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
                self.apply_identity(
                    &mut session,
                    platform_id,
                    platform_user_id,
                    platform_username,
                );
                self.sessions.save_session(&session).await?;
                session
            }
            Some(mut session) => {
                self.apply_identity(
                    &mut session,
                    platform_id,
                    platform_user_id,
                    platform_username,
                );
                self.sessions.save_session(&session).await?;
                session
            }
            None => {
                self.sessions
                    .issue_session_for(
                        target,
                        Some(platform_id),
                        Some(platform_user_id),
                        platform_username,
                        self.sessions.ttl(),
                    )
                    .await?
            }
        };

        let twitch_id = self.twitch_user_id(target).await?;
        let is_admin = match twitch_id.as_deref() {
            Some(id) => self.admin().is_admin(id).await.map_err(admin_error)?,
            None => false,
        };
        if is_admin && platform_id == PlatformId::TWITCH {
            self.admin()
                .update_display_name(platform_user_id, display_name)
                .await
                .ok();
        }
        let is_root = match twitch_id.as_deref() {
            Some(id) => self.admin().is_root(id).await.map_err(admin_error)?,
            None => false,
        };
        Ok((session, is_root))
    }

    pub async fn link(
        &self,
        current_token: Option<&str>,
        login_cookie: Option<&str>,
        ticket: &str,
        merge: bool,
    ) -> Result<(Session, bool), SessionServiceError> {
        let Some(token) = current_token else {
            return Err(SessionServiceError::SessionNotFound);
        };
        let mut session = self.sessions.validate_session(token).await?;
        let platforms = self.users.get_platforms(session.user_id).await?;
        if platforms.is_empty() {
            return self.login(Some(token), login_cookie, ticket).await;
        }
        if login_cookie != Some(ticket) {
            return Err(SessionServiceError::InvalidTicket);
        }
        let ticket = self.sessions.peek_login_ticket(ticket).await?;
        let platform_id = ticket.platform_id;
        let platform_name = platform_id.name();
        let platform_user_id = ticket.platform_user_id.as_str();
        let platform_username = ticket.platform_username.as_deref();
        let display_name = platform_username.unwrap_or(platform_user_id);

        match self
            .users
            .find_by_platform(platform_name, platform_user_id)
            .await?
        {
            Some(existing) if existing.id != session.user_id => {
                if !merge {
                    return Err(SessionServiceError::AccountConflict);
                }
                self.sessions
                    .reassign_user(existing.id, session.user_id)
                    .await?;
                self.users.merge_users(existing.id, session.user_id).await?;
            }
            Some(_) => {}
            None => {
                self.users
                    .link_platform(
                        session.user_id,
                        platform_name,
                        platform_user_id,
                        display_name,
                    )
                    .await?;
            }
        }
        self.sessions
            .consume_login_ticket(ticket.ticket.as_str())
            .await?;
        self.apply_identity(
            &mut session,
            platform_id,
            platform_user_id,
            platform_username,
        );
        self.users
            .update_user(session.user_id, display_name)
            .await
            .ok();
        self.sessions.save_session(&session).await?;

        let twitch_id = self.twitch_user_id(session.user_id).await?;
        let is_root = match twitch_id.as_deref() {
            Some(id) => self.admin().is_root(id).await.map_err(admin_error)?,
            None => false,
        };
        Ok((session, is_root))
    }

    pub async fn twitch_user_id(
        &self,
        user_id: UserId,
    ) -> Result<Option<String>, SessionServiceError> {
        let platforms = self.users.get_platforms(user_id).await?;
        Ok(platforms
            .into_iter()
            .find(|platform| platform.platform_id == PlatformId::TWITCH)
            .map(|platform| platform.platform_user_id))
    }

    fn apply_identity(
        &self,
        session: &mut Session,
        platform_id: PlatformId,
        platform_user_id: &str,
        platform_username: Option<&str>,
    ) {
        session.platform_id = Some(platform_id);
        session.platform_user_id = Some(platform_user_id.to_string());
        session.platform_username = platform_username.map(str::to_string);
    }
}

fn admin_error<E>(_: E) -> SessionServiceError {
    SessionServiceError::Repo(RepositoryError::Database("admin service".into()))
}

#[cfg(test)]
#[path = "account.test.rs"]
mod tests;
