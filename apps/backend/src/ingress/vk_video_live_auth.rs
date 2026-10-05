use chrono::Utc;
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use vk_video_live::api;
use vk_video_live::auth::{OAuthClient, TokenResponse};
use vk_video_live::error::Error as VkError;
use vk_video_live::transport::Transport;

use crate::config::VkVideoLiveConfig;
use crate::error::ingress::PlatformError;
use crate::ingress::platform::{ConnectedIdentity, PlatformAuth};
use crate::platform::{PlatformCredentialRepository, PlatformCredentialService, PlatformId};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct VkCreds {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub user_id: String,
    pub user_nick: String,
    pub channel_id: u64,
    pub channel_url: String,
    #[serde(skip)]
    _sealed: (),
}

impl VkCreds {
    fn from_token(
        token: TokenResponse,
        owner_id: u64,
        owner_nick: &str,
        channel_id: u64,
        channel_url: &str,
    ) -> Self {
        Self {
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_at: Utc::now().timestamp() + token.expires_in as i64,
            user_id: owner_id.to_string(),
            user_nick: owner_nick.to_string(),
            channel_id,
            channel_url: channel_url.to_string(),
            _sealed: (),
        }
    }
}

pub struct VkTransport {
    http: reqwest::Client,
}

impl VkTransport {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for VkTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl VkTransport {
    async fn body(method: &str, url: &str, resp: reqwest::Response) -> Result<String, VkError> {
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| VkError::Http(format!("body read: {e}")))?;
        let redacted = redact_tokens(&text);
        if status.is_success() {
            tracing::debug!(%method, %url, %status, body = %redacted, "vk api response");
            Ok(text)
        } else {
            tracing::warn!(%method, %url, %status, body = %redacted, "vk api error");
            Err(VkError::Http(format!(
                "{method} {url}: status {status}: {redacted}"
            )))
        }
    }
}

fn redact_tokens(body: &str) -> String {
    let mut out = body.to_string();
    for field in ["\"access_token\":\"", "\"refresh_token\":\""] {
        let mut search_from = 0;
        while let Some(rel) = out[search_from..].find(field) {
            let value_start = search_from + rel + field.len();
            let Some(end) = out[value_start..].find('"').map(|i| value_start + i) else {
                break;
            };
            out.replace_range(value_start..end, "***");
            search_from = value_start + 3;
        }
    }
    out
}

impl Transport for VkTransport {
    async fn post_form(
        &self,
        url: &str,
        basic_auth: &str,
        encoded_body: &str,
    ) -> Result<String, VkError> {
        tracing::info!(url, "vk api post form");
        let resp = self
            .http
            .post(url)
            .header(AUTHORIZATION, basic_auth)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(encoded_body.to_string())
            .send()
            .await
            .map_err(|e| VkError::Http(format!("post form: {e}")))?;
        Self::body("POST", url, resp).await
    }

    async fn get(&self, url: &str, bearer: &str) -> Result<String, VkError> {
        tracing::info!(url, "vk api get");
        let resp = self
            .http
            .get(url)
            .header(AUTHORIZATION, bearer)
            .send()
            .await
            .map_err(|e| VkError::Http(format!("get: {e}")))?;
        Self::body("GET", url, resp).await
    }

    async fn post_json(&self, url: &str, bearer: &str, body: &str) -> Result<String, VkError> {
        tracing::info!(url, "vk api post json");
        let resp = self
            .http
            .post(url)
            .header(AUTHORIZATION, bearer)
            .header(CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
            .await
            .map_err(|e| VkError::Http(format!("post json: {e}")))?;
        Self::body("POST", url, resp).await
    }
}

pub struct VkVideoLiveAuthService<R, T = VkTransport>
where
    R: PlatformCredentialRepository,
    T: Transport,
{
    config: Arc<VkVideoLiveConfig>,
    credentials: Arc<PlatformCredentialService<R>>,
    oauth: OAuthClient,
    transport: T,
}

impl<R> VkVideoLiveAuthService<R, VkTransport>
where
    R: PlatformCredentialRepository,
{
    pub fn new(
        config: Arc<VkVideoLiveConfig>,
        credentials: Arc<PlatformCredentialService<R>>,
    ) -> Self {
        Self {
            oauth: OAuthClient::new(config.client_id.clone(), config.client_secret.clone()),
            config,
            credentials,
            transport: VkTransport::new(),
        }
    }
}

impl<R, T> VkVideoLiveAuthService<R, T>
where
    R: PlatformCredentialRepository,
    T: Transport,
{
    pub fn with_transport(
        config: Arc<VkVideoLiveConfig>,
        credentials: Arc<PlatformCredentialService<R>>,
        transport: T,
    ) -> Self {
        Self {
            oauth: OAuthClient::new(config.client_id.clone(), config.client_secret.clone()),
            config,
            credentials,
            transport,
        }
    }

    pub fn oauth(&self) -> &OAuthClient {
        &self.oauth
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn credentials_redirect_uri(&self) -> &str {
        &self.config.credentials_redirect_uri
    }

    pub async fn load(&self) -> Result<Option<VkCreds>, PlatformError> {
        let stored = self
            .credentials
            .load_credential(PlatformId::VK_VIDEO_LIVE)
            .await
            .map_err(|e| PlatformError::Auth(e.to_string()))?;
        match stored.filter(|s| !s.is_empty()) {
            Some(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(|e| PlatformError::Auth(format!("vk creds parse: {e}"))),
            None => Ok(None),
        }
    }

    pub async fn save(&self, creds: &VkCreds) -> Result<(), PlatformError> {
        let raw = serde_json::to_string(creds)
            .map_err(|e| PlatformError::Auth(format!("vk creds serialize: {e}")))?;
        self.credentials
            .save_credential(PlatformId::VK_VIDEO_LIVE, &raw)
            .await
            .map_err(|e| PlatformError::Auth(e.to_string()))
    }

    async fn save_rotated(&self, creds: &VkCreds) -> Result<(), PlatformError> {
        let raw = serde_json::to_string(creds)
            .map_err(|e| PlatformError::Auth(format!("vk creds serialize: {e}")))?;
        self.credentials
            .save_rotated(PlatformId::VK_VIDEO_LIVE, &raw)
            .await
            .map_err(|e| PlatformError::Auth(e.to_string()))
    }

    pub async fn clear(&self) -> Result<(), PlatformError> {
        self.credentials
            .clear_credential(PlatformId::VK_VIDEO_LIVE)
            .await
            .map_err(|e| PlatformError::Auth(e.to_string()))
    }

    pub async fn complete_connect(&self, code: &str) -> Result<VkCreds, PlatformError> {
        tracing::info!("vk connect: exchanging code");
        let token = self
            .oauth
            .exchange_code(&self.transport, code, &self.config.credentials_redirect_uri)
            .await
            .map_err(map_vk)?;
        tracing::info!("vk connect: token exchanged, resolving channel");
        let creds = self.resolve_creds(token).await?;
        tracing::info!(
            channel_id = creds.channel_id,
            "vk connect: channel resolved"
        );
        self.save(&creds).await?;
        Ok(creds)
    }

    async fn resolve_creds(&self, token: TokenResponse) -> Result<VkCreds, PlatformError> {
        let bearer = format!("Bearer {}", token.access_token);
        let user = api::current_user(&self.transport, &bearer)
            .await
            .map_err(map_vk)?;
        let channel_url = user
            .data
            .channel
            .map(|c| c.url)
            .ok_or_else(|| PlatformError::Auth("vk account has no channel".to_string()))?;
        let channel = api::channel(&self.transport, &bearer, &channel_url)
            .await
            .map_err(map_vk)?;
        let (owner_id, owner_nick) = match channel.data.owner.as_ref() {
            Some(owner) => (owner.id, owner.nick.as_str()),
            None => (channel.data.channel.id, channel.data.channel.nick.as_str()),
        };
        Ok(VkCreds::from_token(
            token,
            owner_id,
            owner_nick,
            channel.data.channel.id,
            &channel.data.channel.url,
        ))
    }

    pub async fn access_token(&self) -> Result<String, PlatformError> {
        let mut creds = self
            .load()
            .await?
            .ok_or_else(|| PlatformError::Auth("vk credentials are not configured".to_string()))?;
        if creds.expires_at - 60 <= Utc::now().timestamp() {
            let token = self
                .oauth
                .refresh(
                    &self.transport,
                    &creds.refresh_token,
                    &self.config.credentials_redirect_uri,
                )
                .await
                .map_err(map_vk)?;
            creds.access_token = token.access_token.clone();
            if !token.refresh_token.is_empty() {
                creds.refresh_token = token.refresh_token.clone();
            }
            creds.expires_at = Utc::now().timestamp() + token.expires_in as i64;
            self.save_rotated(&creds).await?;
        }
        Ok(creds.access_token)
    }

    pub async fn bearer(&self) -> Result<String, PlatformError> {
        Ok(format!("Bearer {}", self.access_token().await?))
    }

    pub async fn channel_id(&self) -> Result<u64, PlatformError> {
        self.load()
            .await?
            .ok_or_else(|| PlatformError::Auth("vk credentials are not configured".to_string()))
            .map(|creds| creds.channel_id)
    }

    pub async fn channel_url(&self) -> Result<String, PlatformError> {
        self.load()
            .await?
            .ok_or_else(|| PlatformError::Auth("vk credentials are not configured".to_string()))
            .map(|creds| creds.channel_url)
    }
}

impl<R, T> PlatformAuth for VkVideoLiveAuthService<R, T>
where
    R: PlatformCredentialRepository,
    T: Transport,
{
    fn platform(&self) -> PlatformId {
        PlatformId::VK_VIDEO_LIVE
    }

    async fn is_configured(&self) -> Result<bool, PlatformError> {
        Ok(self.load().await?.is_some())
    }

    async fn connect(&self, code: &str) -> Result<ConnectedIdentity, PlatformError> {
        let creds = self.complete_connect(code).await?;
        Ok(ConnectedIdentity {
            user_id: creds.user_id,
            user_name: creds.user_nick,
        })
    }

    async fn revoke(&self) -> Result<(), PlatformError> {
        self.clear().await
    }
}

fn map_vk(e: VkError) -> PlatformError {
    match e {
        VkError::Http(m) => PlatformError::Auth(format!("vk oauth: {m}")),
        other => PlatformError::Auth(format!("vk: {other}")),
    }
}

#[cfg(test)]
#[path = "vk_video_live_auth.test.rs"]
mod tests;
