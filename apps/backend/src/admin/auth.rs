use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::nonpoison::Mutex;
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use tracing::debug;
use twitch_oauth2::{CsrfToken, Scope, TwitchToken, UserTokenBuilder};

use crate::config::TwitchConfig;
use crate::platform::{PlatformCredentialRepository, PlatformCredentialService, PlatformId};

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
    credentials: Arc<PlatformCredentialService<R>>,
}

impl<R> AdminAuthService<R>
where
    R: PlatformCredentialRepository,
{
    pub fn new(
        config: Option<Arc<TwitchConfig>>,
        credentials: Arc<PlatformCredentialService<R>>,
    ) -> Self {
        Self {
            config,
            pending_csrf: Mutex::new(BTreeMap::new()),
            credentials,
        }
    }

    pub fn start(&self) -> Result<String, AdminAuthError> {
        self.start_with_scopes(ADMIN_SCOPES.to_vec(), |twitch| {
            &twitch.credentials_redirect_uri
        })
    }

    pub fn start_login(&self) -> Result<String, AdminAuthError> {
        self.start_with_scopes(Vec::new(), |twitch| &twitch.redirect_uri)
    }

    fn start_with_scopes(
        &self,
        scopes: Vec<Scope>,
        redirect: impl FnOnce(&TwitchConfig) -> &str,
    ) -> Result<String, AdminAuthError> {
        let twitch = self.config.as_ref().ok_or(AdminAuthError::NotConfigured)?;
        let redirect_url =
            url::Url::parse(redirect(twitch)).map_err(|_| AdminAuthError::InvalidRedirectUri)?;
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
        let token = self
            .exchange(code, auth_state, |twitch| &twitch.credentials_redirect_uri)
            .await?;
        let Some(refresh_token) = token.refresh_token.as_ref() else {
            tracing::error!("twitch oauth returned no refresh token");
            return Err(AdminAuthError::Exchange);
        };
        self.credentials
            .save_credential(PlatformId::TWITCH, refresh_token.secret())
            .await
            .inspect_err(|e| tracing::error!("failed to persist twitch refresh token: {e}"))
            .map_err(|_| AdminAuthError::Persist)?;
        let exchanged = exchanged_of(&token);
        tracing::info!(
            "twitch credentials persisted for backend: twitch_user_id={}",
            exchanged.user_id
        );
        Ok(exchanged)
    }

    pub async fn complete_login(
        &self,
        code: &str,
        auth_state: &str,
    ) -> Result<ExchangedToken, AdminAuthError> {
        let token = self
            .exchange(code, auth_state, |twitch| &twitch.redirect_uri)
            .await?;
        Ok(exchanged_of(&token))
    }

    async fn exchange(
        &self,
        code: &str,
        auth_state: &str,
        redirect: impl FnOnce(&TwitchConfig) -> &str,
    ) -> Result<twitch_oauth2::UserToken, AdminAuthError> {
        let twitch = self.config.as_ref().ok_or(AdminAuthError::NotConfigured)?;
        self.prune_expired();
        if self.pending_csrf.lock().remove(auth_state).is_none() {
            tracing::warn!(
                "twitch oauth csrf mismatch or flow never started (state consumed/expired)"
            );
            return Err(AdminAuthError::CsrfMismatch);
        }

        let redirect_url =
            url::Url::parse(redirect(twitch)).map_err(|_| AdminAuthError::InvalidRedirectUri)?;
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
            .inspect_err(|e| tracing::error!("twitch token exchange failed: {e}"))
            .map_err(|_| AdminAuthError::Exchange)
    }

    pub async fn is_ingress_credentials_configured(&self) -> Result<bool, AdminAuthError> {
        Ok(self
            .credentials
            .load_credential(PlatformId::TWITCH)
            .await
            .map_err(|_| AdminAuthError::Persist)?
            .is_some())
    }

    pub async fn revoke_ingress_credentials(&self) -> Result<(), AdminAuthError> {
        self.credentials
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
    let user_id = token.user_id().map(|u| u.to_string()).unwrap_or_default();
    let user_name = token.login().map(|u| u.to_string());
    debug!("twitch oauth exchanged: twitch_user_id={user_id}, twitch_user_name={user_name:?}");
    ExchangedToken { user_id, user_name }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
    use crate::platform::PlatformCredentialService;

    use super::*;

    fn test_config() -> Option<Arc<TwitchConfig>> {
        Some(Arc::new(TwitchConfig {
            client_id: "client_id".to_string(),
            client_secret: "client_secret".to_string(),
            broadcaster_id: String::new(),
            redirect_uri: "https://localhost:8080/callback".to_string(),
            credentials_redirect_uri: "https://localhost:8080/creds/callback".to_string(),
            csrf_ttl_secs: 600,
        }))
    }

    fn test_service(
        config: Option<Arc<TwitchConfig>>,
    ) -> AdminAuthService<InMemoryPlatformCredentialRepository> {
        AdminAuthService::new(
            config,
            Arc::new(PlatformCredentialService::new(Arc::new(
                InMemoryPlatformCredentialRepository::new(),
            ))),
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

    fn auth_url_redirect_uri(auth_url: &str) -> String {
        url::Url::parse(auth_url)
            .expect("auth url parses")
            .query_pairs()
            .find(|(key, _)| key == "redirect_uri")
            .map(|(_, value)| value.into_owned())
            .expect("auth url carries redirect_uri")
    }

    #[test]
    fn start_uses_credentials_redirect_uri() {
        let service = test_service(test_config());
        let url = service.start().expect("start should succeed");
        assert_eq!(
            auth_url_redirect_uri(&url),
            "https://localhost:8080/creds/callback"
        );
    }

    #[test]
    fn start_login_uses_login_redirect_uri() {
        let service = test_service(test_config());
        let url = service.start_login().expect("start_login should succeed");
        assert_eq!(
            auth_url_redirect_uri(&url),
            "https://localhost:8080/callback"
        );
    }

    #[test]
    fn start_login_requires_twitch_config() {
        let service = test_service(None);
        assert!(matches!(
            service.start_login(),
            Err(AdminAuthError::NotConfigured)
        ));
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
            .credentials
            .save_credential(PlatformId::TWITCH, "tok")
            .await
            .unwrap();
        assert!(service.is_ingress_credentials_configured().await.unwrap());

        service.revoke_ingress_credentials().await.unwrap();
        assert!(!service.is_ingress_credentials_configured().await.unwrap());
    }
}
