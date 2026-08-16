use std::sync::Arc;

use twitch_api::helix::HelixClient;
use twitch_oauth2::{ClientId, ClientSecret, RefreshToken, Scope, TwitchToken, UserToken};

use crate::config::TwitchConfig;
use crate::error::ingress::PlatformError;
use crate::platform::{PlatformCredentialRepository, PlatformCredentialService, PlatformId};

const REQUIRED_SCOPES: &[Scope] = &[Scope::ChatRead, Scope::UserBot];

#[non_exhaustive]
pub struct TwitchAuthService<R>
where
    R: PlatformCredentialRepository,
{
    config: Arc<TwitchConfig>,
    http: reqwest::Client,
    credentials: Arc<PlatformCredentialService<R>>,
}

impl<R> TwitchAuthService<R>
where
    R: PlatformCredentialRepository,
{
    pub fn new(config: Arc<TwitchConfig>, credentials: Arc<PlatformCredentialService<R>>) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
            credentials,
        }
    }

    pub fn helix(&self) -> HelixClient<'static, reqwest::Client> {
        HelixClient::with_client(self.http.clone())
    }

    pub async fn user_token(&self) -> Result<UserToken, PlatformError> {
        let refresh_token = self.current_refresh_token().await?;
        let token = UserToken::from_refresh_token(
            &self.http,
            RefreshToken::new(refresh_token),
            ClientId::new(self.config.client_id.clone()),
            Some(ClientSecret::new(self.config.client_secret.clone())),
        )
        .await
        .map_err(|e| PlatformError::Auth(e.to_string()))?;
        self.persist_rotated(&token).await;
        self.log_scopes(&token);
        Ok(token)
    }

    async fn current_refresh_token(&self) -> Result<String, PlatformError> {
        self.credentials
            .load_credential(PlatformId::TWITCH)
            .await
            .map_err(|e| PlatformError::Auth(e.to_string()))?
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                PlatformError::Auth("twitch refresh token is not configured".to_string())
            })
    }

    async fn persist_rotated(&self, token: &UserToken) {
        let Some(refresh_token) = token.refresh_token.as_ref() else {
            return;
        };
        if let Err(e) = self
            .credentials
            .save_rotated(PlatformId::TWITCH, refresh_token.secret())
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
    use crate::platform::{PlatformCredentialService, PlatformId};

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
            broadcaster_id: "broadcaster".to_string(),
            redirect_uri: String::new(),
            credentials_redirect_uri: String::new(),
            csrf_ttl_secs: 600,
        })
    }

    fn test_credentials() -> Arc<PlatformCredentialService<InMemoryPlatformCredentialRepository>> {
        Arc::new(PlatformCredentialService::new(Arc::new(
            InMemoryPlatformCredentialRepository::new(),
        )))
    }

    #[tokio::test]
    async fn current_refresh_token_reads_from_repo() {
        let credentials = test_credentials();
        credentials
            .save_rotated(PlatformId::TWITCH, "from_repo")
            .await
            .unwrap();
        let service = TwitchAuthService::new(test_config(), credentials);
        assert_eq!(service.current_refresh_token().await.unwrap(), "from_repo");
    }

    #[tokio::test]
    async fn current_refresh_token_errors_when_repo_empty() {
        let service = TwitchAuthService::new(test_config(), test_credentials());
        assert!(matches!(
            service.current_refresh_token().await,
            Err(PlatformError::Auth(_))
        ));
    }

    #[tokio::test]
    async fn current_refresh_token_reads_replaced_credential() {
        let credentials = test_credentials();
        credentials
            .save_credential(PlatformId::TWITCH, "first")
            .await
            .unwrap();
        let service = TwitchAuthService::new(test_config(), credentials);

        assert_eq!(service.current_refresh_token().await.unwrap(), "first");

        service
            .credentials
            .save_credential(PlatformId::TWITCH, "second")
            .await
            .unwrap();
        assert_eq!(service.current_refresh_token().await.unwrap(), "second");
    }

    #[tokio::test]
    async fn save_rotated_does_not_bump_lifecycle() {
        let credentials = test_credentials();
        let rx = credentials.subscribe_lifecycle();
        credentials
            .save_rotated(PlatformId::TWITCH, "rotated")
            .await
            .unwrap();
        assert_eq!(*rx.borrow(), 0);
    }
}
