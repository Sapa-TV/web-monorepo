use std::sync::Arc;
use std::time::Duration;

use vk_video_live::auth::{INGRESS_SCOPES, OAuthClient};
use vk_video_live::transport::Transport;

use crate::admin::auth::AdminAuthError;
use crate::admin::csrf::CsrfStore;
use crate::config::VkVideoLiveConfig;
use crate::ingress::platform::{ConnectedIdentity, PlatformAuth};
use crate::ingress::vk_video_live_auth::{VkTransport, VkVideoLiveAuthService};
use crate::platform::PlatformCredentialRepository;

pub struct VkVideoLiveAdminAuthService<R, T = VkTransport>
where
    R: PlatformCredentialRepository,
    T: Transport,
{
    config: Option<Arc<VkVideoLiveConfig>>,
    csrf: CsrfStore,
    auth: Option<Arc<VkVideoLiveAuthService<R, T>>>,
}

impl<R, T> VkVideoLiveAdminAuthService<R, T>
where
    R: PlatformCredentialRepository,
    T: Transport,
{
    pub fn new(
        config: Option<Arc<VkVideoLiveConfig>>,
        auth: Option<Arc<VkVideoLiveAuthService<R, T>>>,
    ) -> Self {
        Self {
            config,
            csrf: CsrfStore::new(),
            auth,
        }
    }

    pub fn start(&self) -> Result<String, AdminAuthError> {
        let config = self.config.as_ref().ok_or(AdminAuthError::NotConfigured)?;
        self.csrf.prune();
        let state = self.csrf.issue(Duration::from_secs(config.csrf_ttl_secs));
        let oauth = OAuthClient::new(config.client_id.clone(), config.client_secret.clone());
        Ok(oauth.authorize_url(&config.credentials_redirect_uri, INGRESS_SCOPES, &state))
    }

    pub async fn complete(
        &self,
        code: &str,
        state: &str,
    ) -> Result<ConnectedIdentity, AdminAuthError> {
        let auth = self.auth.as_ref().ok_or(AdminAuthError::NotConfigured)?;
        if !self.csrf.consume(state) {
            tracing::warn!("vk oauth csrf mismatch or flow never started (state consumed/expired)");
            return Err(AdminAuthError::CsrfMismatch);
        }
        auth.connect(code).await.map_err(|e| {
            tracing::error!("vk token exchange failed: {e}");
            AdminAuthError::Exchange
        })
    }

    pub async fn is_configured(&self) -> Result<bool, AdminAuthError> {
        let auth = self.auth.as_ref().ok_or(AdminAuthError::NotConfigured)?;
        auth.is_configured()
            .await
            .map_err(|_| AdminAuthError::Persist)
    }

    pub async fn revoke(&self) -> Result<(), AdminAuthError> {
        let auth = self.auth.as_ref().ok_or(AdminAuthError::NotConfigured)?;
        auth.revoke().await.map_err(|_| AdminAuthError::Persist)
    }
}

#[cfg(test)]
#[path = "vk_auth.test.rs"]
mod tests;
