use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::nonpoison::Mutex;
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use twitch_oauth2::{CsrfToken, Scope, TwitchToken, UserTokenBuilder};

use crate::config::TwitchConfig;
use crate::platform::{PlatformCredentialRepository, PlatformId};

const ADMIN_SCOPES: &[Scope] = &[Scope::ChatRead, Scope::UserBot, Scope::ChannelBot];

#[non_exhaustive]
pub struct ExchangedToken {
    pub user_id: String,
    pub user_name: Option<String>,
}

#[non_exhaustive]
#[derive(Debug)]
pub enum AdminAuthError {
    NotConfigured,
    CsrfMismatch,
    InvalidRedirectUri,
    Exchange,
    Persist,
}

impl From<AdminAuthError> for StatusCode {
    fn from(e: AdminAuthError) -> Self {
        match e {
            AdminAuthError::NotConfigured => StatusCode::BAD_REQUEST,
            AdminAuthError::CsrfMismatch => StatusCode::FORBIDDEN,
            AdminAuthError::InvalidRedirectUri
            | AdminAuthError::Exchange
            | AdminAuthError::Persist => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[non_exhaustive]
pub struct AdminAuthService<R>
where
    R: PlatformCredentialRepository,
{
    config: Option<Arc<TwitchConfig>>,
    pending_csrf: Mutex<BTreeMap<String, Instant>>,
    credentials_repo: Arc<R>,
}

impl<R> AdminAuthService<R>
where
    R: PlatformCredentialRepository,
{
    pub fn new(config: Option<Arc<TwitchConfig>>, credentials_repo: Arc<R>) -> Self {
        Self {
            config,
            pending_csrf: Mutex::new(BTreeMap::new()),
            credentials_repo,
        }
    }

    pub fn start(&self) -> Result<String, AdminAuthError> {
        self.start_with_scopes(ADMIN_SCOPES.to_vec())
    }

    pub fn start_login(&self) -> Result<String, AdminAuthError> {
        self.start_with_scopes(Vec::new())
    }

    fn start_with_scopes(&self, scopes: Vec<Scope>) -> Result<String, AdminAuthError> {
        let twitch = self.config.as_ref().ok_or(AdminAuthError::NotConfigured)?;
        let redirect_url = url::Url::parse(&twitch.redirect_uri)
            .map_err(|_| AdminAuthError::InvalidRedirectUri)?;
        let mut builder = UserTokenBuilder::new(
            twitch.client_id.clone(),
            twitch.client_secret.clone(),
            redirect_url,
        )
        .set_scopes(scopes);
        let (auth_url, csrf) = builder.generate_url();
        self.prune_expired();
        let ttl = Duration::from_secs(twitch.csrf_ttl_secs);
        self.pending_csrf
            .lock()
            .insert(csrf.secret().to_string(), Instant::now() + ttl);
        Ok(auth_url.to_string())
    }

    pub async fn complete(
        &self,
        code: &str,
        auth_state: &str,
    ) -> Result<ExchangedToken, AdminAuthError> {
        let token = self.exchange(code, auth_state).await?;
        let Some(refresh_token) = token.refresh_token.as_ref() else {
            return Err(AdminAuthError::Exchange);
        };
        self.credentials_repo
            .save_credential(PlatformId::TWITCH, refresh_token.secret())
            .await
            .map_err(|_| AdminAuthError::Persist)?;
        Ok(exchanged_of(&token))
    }

    pub async fn complete_login(
        &self,
        code: &str,
        auth_state: &str,
    ) -> Result<ExchangedToken, AdminAuthError> {
        let token = self.exchange(code, auth_state).await?;
        Ok(exchanged_of(&token))
    }

    async fn exchange(
        &self,
        code: &str,
        auth_state: &str,
    ) -> Result<twitch_oauth2::UserToken, AdminAuthError> {
        let twitch = self.config.as_ref().ok_or(AdminAuthError::NotConfigured)?;
        self.prune_expired();
        if self.pending_csrf.lock().remove(auth_state).is_none() {
            return Err(AdminAuthError::CsrfMismatch);
        }

        let redirect_url = url::Url::parse(&twitch.redirect_uri)
            .map_err(|_| AdminAuthError::InvalidRedirectUri)?;
        let mut builder = UserTokenBuilder::new(
            twitch.client_id.clone(),
            twitch.client_secret.clone(),
            redirect_url,
        );
        builder.set_csrf(CsrfToken::new(auth_state.to_string()));

        let http = reqwest::Client::new();
        builder
            .get_user_token(&http, auth_state, code)
            .await
            .map_err(|_| AdminAuthError::Exchange)
    }

    pub async fn is_ingress_credentials_configured(&self) -> Result<bool, AdminAuthError> {
        Ok(self
            .credentials_repo
            .load_credential(PlatformId::TWITCH)
            .await
            .map_err(|_| AdminAuthError::Persist)?
            .is_some())
    }

    pub async fn revoke_ingress_credentials(&self) -> Result<(), AdminAuthError> {
        self.credentials_repo
            .clear_credential(PlatformId::TWITCH)
            .await
            .map_err(|_| AdminAuthError::Persist)
    }

    fn prune_expired(&self) {
        let now = Instant::now();
        self.pending_csrf
            .lock()
            .retain(|_, expires_at| *expires_at > now);
    }
}

fn exchanged_of(token: &twitch_oauth2::UserToken) -> ExchangedToken {
    ExchangedToken {
        user_id: token.user_id().map(|u| u.to_string()).unwrap_or_default(),
        user_name: token.login().map(|u| u.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;

    use super::*;

    fn test_config() -> Option<Arc<TwitchConfig>> {
        Some(Arc::new(TwitchConfig {
            client_id: "client_id".to_string(),
            client_secret: "client_secret".to_string(),
            refresh_token: String::new(),
            broadcaster_id: String::new(),
            redirect_uri: "https://localhost:8080/callback".to_string(),
            csrf_ttl_secs: 600,
        }))
    }

    fn test_service(
        config: Option<Arc<TwitchConfig>>,
    ) -> AdminAuthService<InMemoryPlatformCredentialRepository> {
        AdminAuthService::new(
            config,
            Arc::new(InMemoryPlatformCredentialRepository::new()),
        )
    }

    #[test]
    fn start_requires_twitch_config() {
        let service = test_service(None);
        assert!(matches!(
            service.start(),
            Err(AdminAuthError::NotConfigured)
        ));
    }

    #[test]
    fn start_returns_redirect_url() {
        let service = test_service(test_config());
        let url = service.start().expect("start should succeed");
        assert!(url.starts_with("https://id.twitch.tv/oauth2/authorize"));
    }

    #[test]
    fn concurrent_starts_keep_own_csrf_tickets() {
        let service = test_service(test_config());
        let first = service.start().expect("first start");
        let second = service.start().expect("second start");
        assert_ne!(first, second, "each start must mint its own ticket");

        assert_eq!(service.pending_csrf.lock().len(), 2);
    }

    #[test]
    fn completing_consumes_only_own_ticket() {
        let service = test_service(test_config());
        service.start().expect("first start");
        service.start().expect("second start");

        let first_ticket = service
            .pending_csrf
            .lock()
            .first_key_value()
            .map(|(ticket, _)| ticket.clone())
            .expect("has ticket");
        assert!(service.pending_csrf.lock().remove(&first_ticket).is_some());
        assert_eq!(service.pending_csrf.lock().len(), 1);
    }

    #[tokio::test]
    async fn unknown_state_is_rejected() {
        let service = test_service(test_config());
        service.start().expect("start");
        assert!(matches!(
            service.complete("code", "not-a-real-state").await,
            Err(AdminAuthError::CsrfMismatch)
        ));
    }

    #[tokio::test]
    async fn expired_ticket_is_pruned_on_complete() {
        let service = test_service(test_config());
        service
            .pending_csrf
            .lock()
            .insert("stale".to_string(), Instant::now() - Duration::from_secs(1));
        assert!(matches!(
            service.complete("code", "stale").await,
            Err(AdminAuthError::CsrfMismatch)
        ));
        assert!(service.pending_csrf.lock().is_empty());
    }

    #[tokio::test]
    async fn ingress_credentials_configuration_lifecycle() {
        let service = test_service(test_config());
        assert!(!service.is_ingress_credentials_configured().await.unwrap());

        service
            .credentials_repo
            .save_credential(PlatformId::TWITCH, "tok")
            .await
            .unwrap();
        assert!(service.is_ingress_credentials_configured().await.unwrap());

        service.revoke_ingress_credentials().await.unwrap();
        assert!(!service.is_ingress_credentials_configured().await.unwrap());
    }
}
