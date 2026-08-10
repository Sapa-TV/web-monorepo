use std::sync::Arc;

use tokio::sync::Mutex;
use twitch_api::helix::HelixClient;
use twitch_oauth2::{ClientId, ClientSecret, RefreshToken, Scope, TwitchToken, UserToken};

use crate::config::TwitchConfig;
use crate::error::RepositoryError;
use crate::error::ingress::PlatformError;
use crate::platform::{PlatformCredentialRepository, PlatformId};

const REQUIRED_SCOPES: &[Scope] = &[Scope::ChatRead, Scope::UserBot];

#[non_exhaustive]
pub struct TwitchAuthService<R>
where
    R: PlatformCredentialRepository,
{
    config: Arc<TwitchConfig>,
    http: reqwest::Client,
    token: Mutex<Option<UserToken>>,
    credentials_repo: Arc<R>,
}

impl<R> TwitchAuthService<R>
where
    R: PlatformCredentialRepository,
{
    pub fn new(config: Arc<TwitchConfig>, credentials_repo: Arc<R>) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            token: Mutex::new(None),
            credentials_repo,
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
                self.persist_rotated(token).await;
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
        let refresh_token = self
            .current_refresh_token()
            .await
            .map_err(|e| PlatformError::Auth(e.to_string()))?;
        let token = UserToken::from_refresh_token(
            &self.http,
            RefreshToken::new(refresh_token),
            ClientId::new(self.config.client_id.clone()),
            Some(ClientSecret::new(self.config.client_secret.clone())),
        )
        .await
        .map_err(|e| PlatformError::Auth(e.to_string()))?;
        self.persist_rotated(&token).await;
        Ok(token)
    }

    async fn current_refresh_token(&self) -> Result<String, RepositoryError> {
        Ok(self
            .credentials_repo
            .load_credential(PlatformId::TWITCH)
            .await?
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.config.refresh_token.clone()))
    }

    async fn persist_rotated(&self, token: &UserToken) {
        let Some(refresh_token) = token.refresh_token.as_ref() else {
            return;
        };
        if let Err(e) = self
            .credentials_repo
            .save_credential(PlatformId::TWITCH, refresh_token.secret())
            .await
        {
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
    use std::sync::Arc;

    use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
    use crate::platform::PlatformId;

    use super::*;

    #[test]
    fn required_scopes_cover_chat_reading() {
        assert!(REQUIRED_SCOPES.contains(&Scope::ChatRead));
        assert!(REQUIRED_SCOPES.contains(&Scope::UserBot));
    }

    fn test_config() -> Arc<TwitchConfig> {
        Arc::new(TwitchConfig {
            client_id: "id".to_string(),
            client_secret: "secret".to_string(),
            refresh_token: "from_config".to_string(),
            broadcaster_id: "broadcaster".to_string(),
            redirect_uri: String::new(),
            csrf_ttl_secs: 600,
        })
    }

    #[tokio::test]
    async fn current_refresh_token_prefers_repo_over_config() {
        let repo = Arc::new(InMemoryPlatformCredentialRepository::new());
        repo.save_credential(PlatformId::TWITCH, "from_repo")
            .await
            .unwrap();
        let service = TwitchAuthService::new(test_config(), repo);
        assert_eq!(service.current_refresh_token().await.unwrap(), "from_repo");
    }

    #[tokio::test]
    async fn current_refresh_token_falls_back_to_config_when_repo_empty() {
        let repo = Arc::new(InMemoryPlatformCredentialRepository::new());
        let service = TwitchAuthService::new(test_config(), repo);
        assert_eq!(
            service.current_refresh_token().await.unwrap(),
            "from_config"
        );
    }
}
