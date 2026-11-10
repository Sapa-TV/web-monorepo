use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::nonpoison::Mutex;
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::random;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

use crate::admin::auth::AdminAuthError;
use crate::config::VkIdConfig;
use domain::session::exchange::{ExchangedToken, LoginExchange};
use model::platform::PlatformId;

struct Pending {
    code_verifier: String,
    device_id: String,
    expires_at: Instant,
}

#[non_exhaustive]
pub struct VkIdAuthService {
    config: Option<Arc<VkIdConfig>>,
    client: reqwest::Client,
    pending: Mutex<BTreeMap<String, Pending>>,
}

impl VkIdAuthService {
    pub fn new(config: Option<Arc<VkIdConfig>>) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            pending: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn start_login(&self) -> Result<String, AdminAuthError> {
        let config = self.config.as_ref().ok_or(AdminAuthError::NotConfigured)?;
        let state = format!("{:032x}", random::<u128>());
        let code_verifier = random_verifier();
        let code_challenge = pkce_challenge(&code_verifier);
        let device_id = Uuid::now_v7().to_string();
        let expires_at = Instant::now() + Duration::from_secs(config.csrf_ttl_secs);

        self.pending.lock().insert(
            state.clone(),
            Pending {
                code_verifier,
                device_id: device_id.clone(),
                expires_at,
            },
        );
        self.prune();

        let base = config.api_base_url.trim_end_matches('/');
        let mut url = Url::parse(&format!("{base}/authorize"))
            .map_err(|_| AdminAuthError::InvalidRedirectUri)?;
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &config.client_id)
            .append_pair("redirect_uri", &config.redirect_uri)
            .append_pair("state", &state)
            .append_pair("code_challenge", &code_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("device_id", &device_id);
        Ok(url.to_string())
    }

    async fn exchange(
        &self,
        code: &str,
        auth_state: &str,
    ) -> Result<ExchangedToken, AdminAuthError> {
        let config = self.config.as_ref().ok_or(AdminAuthError::NotConfigured)?;
        let pending = self
            .consume(auth_state)
            .ok_or(AdminAuthError::CsrfMismatch)?;
        let base = config.api_base_url.trim_end_matches('/');

        let token: TokenResponse = self
            .client
            .post(format!("{base}/oauth2/auth"))
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("code_verifier", pending.code_verifier.as_str()),
                ("client_id", config.client_id.as_str()),
                ("client_secret", config.client_secret.as_str()),
                ("device_id", pending.device_id.as_str()),
                ("redirect_uri", config.redirect_uri.as_str()),
            ])
            .send()
            .await
            .map_err(log_exchange)?
            .error_for_status()
            .map_err(log_exchange)?
            .json()
            .await
            .map_err(log_exchange)?;

        let token_user_id = json_id(token.user_id.as_ref()).unwrap_or_default();
        let user = self
            .user_info(config, &token.access_token, &token_user_id)
            .await
            .unwrap_or_default();
        let user_id = user.id.or(Some(token_user_id)).unwrap_or_default();
        if user_id.is_empty() {
            return Err(AdminAuthError::Exchange);
        }
        Ok(ExchangedToken::new(PlatformId::VK, user_id, user.name))
    }

    async fn user_info(
        &self,
        config: &VkIdConfig,
        access_token: &str,
        fallback_id: &str,
    ) -> Result<VkUser, AdminAuthError> {
        let base = config.api_base_url.trim_end_matches('/');
        let response: UserInfoResponse = self
            .client
            .post(format!("{base}/oauth2/user_info"))
            .form(&[
                ("access_token", access_token),
                ("client_id", config.client_id.as_str()),
            ])
            .send()
            .await
            .map_err(log_exchange)?
            .error_for_status()
            .map_err(log_exchange)?
            .json()
            .await
            .map_err(log_exchange)?;
        let Some(user) = response.user else {
            return Ok(VkUser {
                id: Some(fallback_id.to_string()),
                name: None,
            });
        };
        let id = json_id(user.user_id.as_ref()).filter(|id| !id.is_empty());
        let name = [user.first_name, user.last_name]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        Ok(VkUser {
            id,
            name: (!name.is_empty()).then_some(name),
        })
    }

    fn consume(&self, state: &str) -> Option<Pending> {
        self.prune();
        self.pending.lock().remove(state)
    }

    fn prune(&self) {
        let now = Instant::now();
        self.pending
            .lock()
            .retain(|_, pending| pending.expires_at > now);
    }
}

impl LoginExchange for VkIdAuthService {
    async fn complete_login(&self, code: &str, auth_state: &str) -> Result<ExchangedToken, String> {
        self.exchange(code, auth_state)
            .await
            .map_err(|e| e.to_string())
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    user_id: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct UserInfoResponse {
    #[serde(default)]
    user: Option<VkUserRaw>,
}

#[derive(Deserialize)]
struct VkUserRaw {
    #[serde(default)]
    user_id: Option<serde_json::Value>,
    #[serde(default)]
    first_name: Option<String>,
    #[serde(default)]
    last_name: Option<String>,
}

#[derive(Default)]
struct VkUser {
    id: Option<String>,
    name: Option<String>,
}

fn random_verifier() -> String {
    URL_SAFE_NO_PAD.encode(random::<[u8; 48]>())
}

fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn json_id(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(id) => Some(id.clone()),
        serde_json::Value::Number(id) => Some(id.to_string()),
        _ => None,
    }
}

fn log_exchange(error: reqwest::Error) -> AdminAuthError {
    tracing::error!("vk id exchange failed: {error}");
    AdminAuthError::Exchange
}

#[cfg(test)]
#[path = "vk_id_auth.test.rs"]
mod tests;
