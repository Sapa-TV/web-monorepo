use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use thiserror::Error;
use tracing::debug;
use twitch_oauth2::{CsrfToken, Scope, TwitchToken, UserTokenBuilder};

use crate::admin::csrf::CsrfStore;
use crate::config::TwitchConfig;
use crate::ingress::twitch_auth::INGRESS_SCOPES;
use crate::platform::{PlatformCredentialRepository, PlatformCredentialService, PlatformId};

#[non_exhaustive]
pub struct ExchangedToken {
    pub user_id: String,
    pub user_name: Option<String>,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AdminAuthError {
    #[error("twitch auth is not configured")]
    NotConfigured,
    #[error("csrf state mismatch")]
    CsrfMismatch,
    #[error("invalid redirect uri")]
    InvalidRedirectUri,
    #[error("token exchange failed")]
    Exchange,
    #[error("failed to persist credentials")]
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
    csrf: CsrfStore,
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
            csrf: CsrfStore::new(),
            credentials,
        }
    }

    pub fn start(&self) -> Result<String, AdminAuthError> {
        self.start_with_scopes(INGRESS_SCOPES.to_vec(), |twitch| {
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
        self.csrf.prune();
        let ttl = Duration::from_secs(twitch.csrf_ttl_secs);
        self.csrf
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
        if !self.csrf.consume(auth_state) {
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
}

fn exchanged_of(token: &twitch_oauth2::UserToken) -> ExchangedToken {
    let user_id = token.user_id().map(|u| u.to_string()).unwrap_or_default();
    let user_name = token.login().map(|u| u.to_string());
    debug!("twitch oauth exchanged: twitch_user_id={user_id}, twitch_user_name={user_name:?}");
    ExchangedToken { user_id, user_name }
}

#[cfg(test)]
#[path = "auth.test.rs"]
mod tests;
