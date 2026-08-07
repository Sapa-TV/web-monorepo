use std::collections::BTreeMap;
use std::sync::nonpoison::Mutex;
use std::time::{Duration, Instant};

use twitch_oauth2::{CsrfToken, Scope, TwitchToken, UserTokenBuilder};

use crate::config::TwitchConfig;
use crate::ingress::twitch_auth::TwitchRefreshTokenStore;

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

#[non_exhaustive]
pub struct AdminAuthService {
    config: Option<TwitchConfig>,
    pending_csrf: Mutex<BTreeMap<String, Instant>>,
    refresh_token_store: TwitchRefreshTokenStore,
}

impl AdminAuthService {
    pub fn new(config: Option<TwitchConfig>) -> Self {
        Self::with_refresh_token_store(config, TwitchRefreshTokenStore::default())
    }

    pub fn with_refresh_token_store(
        config: Option<TwitchConfig>,
        refresh_token_store: TwitchRefreshTokenStore,
    ) -> Self {
        Self {
            config,
            pending_csrf: Mutex::new(BTreeMap::new()),
            refresh_token_store,
        }
    }

    pub fn start(&self) -> Result<String, AdminAuthError> {
        let twitch = self.config.as_ref().ok_or(AdminAuthError::NotConfigured)?;
        let redirect_url = url::Url::parse(&twitch.redirect_uri)
            .map_err(|_| AdminAuthError::InvalidRedirectUri)?;
        let mut builder = UserTokenBuilder::new(
            twitch.client_id.clone(),
            twitch.client_secret.clone(),
            redirect_url,
        )
        .set_scopes(ADMIN_SCOPES.to_vec());
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
        let token = builder
            .get_user_token(&http, auth_state, code)
            .await
            .map_err(|_| AdminAuthError::Exchange)?;

        let Some(refresh_token) = token.refresh_token.as_ref() else {
            return Err(AdminAuthError::Exchange);
        };
        self.refresh_token_store
            .save(refresh_token.secret())
            .map_err(|_| AdminAuthError::Persist)?;

        Ok(ExchangedToken {
            user_id: token.user_id().map(|u| u.to_string()).unwrap_or_default(),
            user_name: token.login().map(|u| u.to_string()),
        })
    }

    fn prune_expired(&self) {
        let now = Instant::now();
        self.pending_csrf
            .lock()
            .retain(|_, expires_at| *expires_at > now);
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    fn test_config() -> Option<TwitchConfig> {
        Some(TwitchConfig {
            client_id: "client_id".to_string(),
            client_secret: "client_secret".to_string(),
            refresh_token: String::new(),
            broadcaster_id: String::new(),
            redirect_uri: "https://localhost:8080/callback".to_string(),
            csrf_ttl_secs: 600,
        })
    }

    #[test]
    fn start_requires_twitch_config() {
        let service = AdminAuthService::new(None);
        assert!(matches!(
            service.start(),
            Err(AdminAuthError::NotConfigured)
        ));
    }

    #[test]
    fn start_returns_redirect_url() {
        let service = AdminAuthService::new(test_config());
        let url = service.start().expect("start should succeed");
        assert!(url.starts_with("https://id.twitch.tv/oauth2/authorize"));
    }

    #[test]
    fn concurrent_starts_keep_own_csrf_tickets() {
        let service = AdminAuthService::new(test_config());
        let first = service.start().expect("first start");
        let second = service.start().expect("second start");
        assert_ne!(first, second, "each start must mint its own ticket");

        assert_eq!(service.pending_csrf.lock().len(), 2);
    }

    #[test]
    fn completing_consumes_only_own_ticket() {
        let service = AdminAuthService::new(test_config());
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
        let service = AdminAuthService::new(test_config());
        service.start().expect("start");
        assert!(matches!(
            service.complete("code", "not-a-real-state").await,
            Err(AdminAuthError::CsrfMismatch)
        ));
    }

    #[tokio::test]
    async fn expired_ticket_is_pruned_on_complete() {
        let service = AdminAuthService::new(test_config());
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
}
