use std::fs;
use std::path::PathBuf;

use tokio::sync::Mutex;
use twitch_api::helix::HelixClient;
use twitch_oauth2::{ClientId, ClientSecret, RefreshToken, Scope, TwitchToken, UserToken};

use crate::config::TwitchConfig;
use crate::error::ingress::PlatformError;

const REQUIRED_SCOPES: &[Scope] = &[Scope::ChatRead, Scope::UserBot];
const DEFAULT_REFRESH_TOKEN_PATH: &str = "twitch_refresh_token";

#[derive(Debug)]
#[non_exhaustive]
pub struct TwitchRefreshTokenStore {
    path: PathBuf,
}

impl Default for TwitchRefreshTokenStore {
    fn default() -> Self {
        Self::new(DEFAULT_REFRESH_TOKEN_PATH)
    }
}

impl TwitchRefreshTokenStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Option<String> {
        fs::read_to_string(&self.path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn save(&self, refresh_token: &str) -> Result<(), PlatformError> {
        fs::write(&self.path, refresh_token.trim()).map_err(|e| {
            PlatformError::Auth(format!("failed to persist twitch refresh token: {e}"))
        })
    }
}

#[non_exhaustive]
pub struct TwitchAuthService {
    config: TwitchConfig,
    http: reqwest::Client,
    token: Mutex<Option<UserToken>>,
    refresh_token_store: TwitchRefreshTokenStore,
}

impl TwitchAuthService {
    pub fn new(config: TwitchConfig) -> Self {
        Self::with_refresh_token_store(config, TwitchRefreshTokenStore::default())
    }

    pub fn with_refresh_token_store(
        config: TwitchConfig,
        refresh_token_store: TwitchRefreshTokenStore,
    ) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            token: Mutex::new(None),
            refresh_token_store,
        }
    }

    pub fn helix(&self) -> HelixClient<'static, reqwest::Client> {
        HelixClient::with_client(self.http.clone())
    }

    pub async fn user_token(&self) -> Result<UserToken, PlatformError> {
        let mut cached = self.token.lock().await;
        match &mut *cached {
            Some(token) if !token.is_elapsed() => Ok(token.clone()),
            Some(_) => {
                tracing::warn!("twitch token expired, refreshing");
                let token = cached.as_mut().expect("token is set");
                token
                    .refresh_token(&self.http)
                    .await
                    .map_err(|e| PlatformError::Auth(e.to_string()))?;
                self.persist_rotated(token);
                let token = cached.as_ref().expect("token is set").clone();
                self.log_scopes(&token);
                Ok(token)
            }
            None => {
                let token = self.refresh().await?;
                self.log_scopes(&token);
                *cached = Some(token.clone());
                Ok(token)
            }
        }
    }

    async fn refresh(&self) -> Result<UserToken, PlatformError> {
        let token = UserToken::from_refresh_token(
            &self.http,
            RefreshToken::new(self.current_refresh_token()),
            ClientId::new(self.config.client_id.clone()),
            Some(ClientSecret::new(self.config.client_secret.clone())),
        )
        .await
        .map_err(|e| PlatformError::Auth(e.to_string()))?;
        self.persist_rotated(&token);
        Ok(token)
    }

    fn current_refresh_token(&self) -> String {
        self.refresh_token_store
            .load()
            .unwrap_or_else(|| self.config.refresh_token.clone())
    }

    fn persist_rotated(&self, token: &UserToken) {
        let Some(refresh_token) = token.refresh_token.as_ref() else {
            return;
        };
        if let Err(e) = self.refresh_token_store.save(refresh_token.secret()) {
            tracing::warn!("{e}");
        }
    }

    fn log_scopes(&self, token: &UserToken) {
        let scopes = token.scopes();
        for required in REQUIRED_SCOPES {
            if !scopes.contains(required) {
                tracing::warn!(
                    "twitch token is missing required scope {required:?}, channel.chat.message may not fire"
                );
            }
        }
        tracing::info!(
            login = ?token.login(),
            user_id = ?token.user_id(),
            scopes = ?scopes,
            "twitch token validated"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn required_scopes_cover_chat_reading() {
        assert!(REQUIRED_SCOPES.contains(&Scope::ChatRead));
        assert!(REQUIRED_SCOPES.contains(&Scope::UserBot));
    }

    fn temp_store_path(suffix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is sane")
            .as_nanos();
        env::temp_dir().join(format!("twitch_refresh_token_{suffix}_{nanos}"))
    }

    #[test]
    fn store_roundtrip() {
        let path = temp_store_path("roundtrip");
        let store = TwitchRefreshTokenStore::new(&path);
        assert!(store.load().is_none());
        store.save("refresh_token_1").unwrap();
        assert_eq!(store.load().as_deref(), Some("refresh_token_1"));
        drop(fs::remove_file(&path));
    }

    #[test]
    fn store_prefers_file_over_config_empty() {
        let path = temp_store_path("prefers_file");
        let store = TwitchRefreshTokenStore::new(&path);
        store.save("from_file").unwrap();
        let config = TwitchConfig {
            client_id: "id".to_string(),
            client_secret: "secret".to_string(),
            refresh_token: String::new(),
            broadcaster_id: "broadcaster".to_string(),
            redirect_uri: String::new(),
            csrf_ttl_secs: 600,
        };
        let service = TwitchAuthService::with_refresh_token_store(config, store);
        assert_eq!(service.current_refresh_token(), "from_file");
        drop(fs::remove_file(&path));
    }
}
