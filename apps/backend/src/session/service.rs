use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::admin::repository::AdminRepository;
use crate::admin::service::AdminService;
use crate::config::store::SharedSettings;
use crate::consts::session::LOGIN_TICKET_TTL;
use crate::error::RepositoryError;
use crate::error::SessionServiceError;
use crate::session::repository::SessionRepository;
use crate::session::{LoginTicket, LoginTicketToken, Session, SessionToken};

#[non_exhaustive]
pub struct SessionService<R>
where
    R: SessionRepository,
{
    repo: Arc<R>,
    settings: SharedSettings,
}

impl<R> SessionService<R>
where
    R: SessionRepository,
{
    pub fn new(repo: Arc<R>, settings: SharedSettings) -> Self {
        Self { repo, settings }
    }

    pub async fn create_login_ticket(
        &self,
        twitch_user_id: &str,
        twitch_user_name: Option<&str>,
    ) -> Result<LoginTicket, SessionServiceError> {
        let now = Utc::now();
        let ticket = LoginTicket {
            ticket: LoginTicketToken::new(nonce()),
            twitch_user_id: twitch_user_id.to_string(),
            twitch_user_name: twitch_user_name.map(str::to_string),
            created_at: now,
            expires_at: now + LOGIN_TICKET_TTL,
        };
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
        let ttl = Duration::from_secs(self.settings.read().session_ttl_secs);
        let session = Session {
            token: SessionToken::new(nonce()),
            twitch_user_id: twitch_user_id.to_string(),
            twitch_user_name: twitch_user_name.map(str::to_string),
            created_at: now,
            expires_at: now + ttl,
        };
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

    pub async fn login<A>(
        &self,
        admin: &AdminService<A>,
        login_cookie: Option<&str>,
        ticket: &str,
    ) -> Result<(Session, bool), SessionServiceError>
    where
        A: AdminRepository,
    {
        if login_cookie != Some(ticket) {
            return Err(SessionServiceError::InvalidTicket);
        }

        let ticket = self.consume_login_ticket(ticket).await?;

        let is_admin = admin.is_admin(&ticket.twitch_user_id).await.map_err(|_| {
            SessionServiceError::Repo(RepositoryError::Database("admin service".to_string()))
        })?;
        tracing::debug!(
            "login: twitch_user_id={}, is_admin={}",
            ticket.twitch_user_id,
            is_admin
        );
        if is_admin {
            admin
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

        let is_root = admin.is_root(&session.twitch_user_id).await.map_err(|_| {
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
mod tests {
    use std::sync::Arc;

    use crate::admin::service::AdminService;
    use crate::config::runtime::RuntimeConfig;
    use crate::db::inmemory_admin::InMemoryAdminRepository;
    use crate::db::inmemory_session::InMemorySessionRepository;

    use super::*;

    type TestService = SessionService<InMemorySessionRepository>;

    async fn admin_service() -> AdminService<InMemoryAdminRepository> {
        AdminService::new(Arc::new(InMemoryAdminRepository::new()))
    }

    fn test_settings(ttl_secs: u64) -> SharedSettings {
        let mut config = RuntimeConfig::test_runtime("test-key");
        config.session_ttl_secs = ttl_secs;
        SharedSettings::test_new(config)
    }

    fn test_service() -> TestService {
        SessionService::new(
            Arc::new(InMemorySessionRepository::new()),
            test_settings(60 * 60),
        )
    }

    #[tokio::test]
    async fn ticket_is_one_time_use() {
        let svc = test_service();
        let ticket = svc
            .create_login_ticket("123", Some("sapushka_"))
            .await
            .unwrap();

        let consumed = svc
            .consume_login_ticket(ticket.ticket.as_str())
            .await
            .unwrap();
        assert_eq!(consumed.twitch_user_id, "123");
        assert_eq!(consumed.twitch_user_name.as_deref(), Some("sapushka_"));

        let err = svc
            .consume_login_ticket(ticket.ticket.as_str())
            .await
            .unwrap_err();
        assert!(matches!(err, SessionServiceError::InvalidTicket));
    }

    #[tokio::test]
    async fn unknown_ticket_is_rejected() {
        let svc = test_service();
        let err = svc.consume_login_ticket("bogus").await.unwrap_err();
        assert!(matches!(err, SessionServiceError::InvalidTicket));
    }

    #[tokio::test]
    async fn login_ticket_creates_a_session() {
        let svc = test_service();
        let ticket = svc.create_login_ticket("123", None).await.unwrap();
        let consumed = svc
            .consume_login_ticket(ticket.ticket.as_str())
            .await
            .unwrap();

        let session = svc
            .issue_session(
                &consumed.twitch_user_id,
                consumed.twitch_user_name.as_deref(),
            )
            .await
            .unwrap();
        assert_eq!(session.twitch_user_id, "123");
        assert_eq!(session.token.as_str().len(), 64);

        let validated = svc.validate_session(session.token.as_str()).await.unwrap();
        assert_eq!(validated.twitch_user_id, "123");
    }

    #[tokio::test]
    async fn validate_rejects_unknown_session() {
        let svc = test_service();
        let err = svc.validate_session("nope").await.unwrap_err();
        assert!(matches!(err, SessionServiceError::SessionNotFound));
    }

    #[tokio::test]
    async fn logout_invalidates_session() {
        let svc = test_service();
        let session = svc.issue_session("123", None).await.unwrap();
        svc.logout(session.token.as_str()).await.unwrap();
        let err = svc
            .validate_session(session.token.as_str())
            .await
            .unwrap_err();
        assert!(matches!(err, SessionServiceError::SessionNotFound));
    }

    async fn login_ticket(svc: &TestService, twitch_id: &str, name: Option<&str>) -> String {
        svc.create_login_ticket(twitch_id, name)
            .await
            .unwrap()
            .ticket
            .as_str()
            .to_string()
    }

    #[tokio::test]
    async fn login_updates_display_name_for_admin() {
        let svc = test_service();
        let admins = admin_service().await;
        admins.add("123", Some("old_name")).await.unwrap();

        let ticket = login_ticket(&svc, "123", Some("new_name")).await;
        let (session, is_root) = svc
            .login(&admins, Some(ticket.as_str()), ticket.as_str())
            .await
            .unwrap();

        assert_eq!(session.twitch_user_id, "123");
        assert_eq!(session.twitch_user_name.as_deref(), Some("new_name"));
        assert!(!is_root);
        assert_eq!(
            admins
                .get("123")
                .await
                .unwrap()
                .unwrap()
                .display_name
                .as_deref(),
            Some("new_name")
        );
    }

    #[tokio::test]
    async fn login_allows_regular_user() {
        let svc = test_service();
        let admins = admin_service().await;

        let ticket = login_ticket(&svc, "999", Some("viewer")).await;
        let (session, is_root) = svc
            .login(&admins, Some(ticket.as_str()), ticket.as_str())
            .await
            .unwrap();

        assert_eq!(session.twitch_user_id, "999");
        assert_eq!(session.twitch_user_name.as_deref(), Some("viewer"));
        assert!(!is_root);
        assert!(admins.get("999").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn login_reports_is_root_for_root_admin() {
        let svc = test_service();
        let admins = admin_service().await;
        admins.add("123", None).await.unwrap();
        admins.set_root("123", true).await.unwrap();

        let ticket = login_ticket(&svc, "123", None).await;
        let (session, is_root) = svc
            .login(&admins, Some(ticket.as_str()), ticket.as_str())
            .await
            .unwrap();

        assert_eq!(session.twitch_user_id, "123");
        assert!(is_root);
    }

    #[tokio::test]
    async fn login_rejects_ticket_without_matching_cookie() {
        let svc = test_service();
        let admins = admin_service().await;
        let ticket = login_ticket(&svc, "123", None).await;

        let err = svc.login(&admins, None, ticket.as_str()).await.unwrap_err();
        assert!(matches!(err, SessionServiceError::InvalidTicket));

        let err = svc
            .login(&admins, Some("stolen"), ticket.as_str())
            .await
            .unwrap_err();
        assert!(matches!(err, SessionServiceError::InvalidTicket));
    }

    #[tokio::test]
    async fn expired_sessions_are_pruned() {
        let repo = Arc::new(InMemorySessionRepository::new());
        let svc = SessionService::new(Arc::clone(&repo), test_settings(60));

        let fresh = svc.issue_session("123", None).await.unwrap();
        let stale = Session {
            token: SessionToken::new("stale"),
            twitch_user_id: "9".to_string(),
            twitch_user_name: None,
            created_at: Utc::now() - Duration::from_secs(3600),
            expires_at: Utc::now() - Duration::from_secs(60),
        };
        repo.save_session(&stale).await.unwrap();

        assert_eq!(svc.prune_expired().await.unwrap(), 1);
        assert!(svc.validate_session(fresh.token.as_str()).await.is_ok());
        let err = svc.validate_session("stale").await.unwrap_err();
        assert!(matches!(err, SessionServiceError::SessionNotFound));
    }

    #[tokio::test]
    async fn session_ttl_follows_live_settings() {
        use crate::config::static_config::StaticConfig;
        use crate::config::store::ConfigStore;
        use crate::db::inmemory_config::InMemoryConfigRepository;

        let repo = Arc::new(InMemoryConfigRepository::new());
        let store = ConfigStore::new(
            Arc::new(StaticConfig::test_config()),
            RuntimeConfig::test_runtime("test-key"),
            Arc::clone(&repo),
        );
        let svc = SessionService::new(Arc::new(InMemorySessionRepository::new()), store.source());

        let first = svc.issue_session("123", None).await.unwrap();
        let first_ttl = first
            .expires_at
            .signed_duration_since(first.created_at)
            .num_seconds();

        let mut next = RuntimeConfig::test_runtime("test-key");
        next.session_ttl_secs = 30;
        store.update_runtime(next).await.unwrap();

        let second = svc.issue_session("456", None).await.unwrap();
        let second_ttl = second
            .expires_at
            .signed_duration_since(second.created_at)
            .num_seconds();

        assert_eq!(first_ttl, 24 * 60 * 60);
        assert_eq!(second_ttl, 30);
    }
}
