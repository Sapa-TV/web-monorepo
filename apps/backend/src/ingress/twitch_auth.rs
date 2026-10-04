use std::sync::Arc;

use futures_util::StreamExt;
use twitch_api::helix::HelixClient;
use twitch_api::helix::points::CustomReward;
use twitch_api::helix::users::User;
use twitch_api::types::{self, Collection, RewardId};
use twitch_oauth2::{ClientId, ClientSecret, RefreshToken, Scope, UserToken};

use crate::config::TwitchConfig;
use crate::error::ingress::PlatformError;
use crate::platform::{PlatformCredentialRepository, PlatformCredentialService, PlatformId};

pub(crate) const INGRESS_SCOPES: &[Scope] = &[
    Scope::ChatRead,
    Scope::UserBot,
    Scope::ChannelBot,
    Scope::UserReadChat,
    Scope::ChannelReadRedemptions,
    Scope::UserWriteChat,
];

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
        Ok(token)
    }

    pub async fn custom_rewards(&self) -> Result<Vec<CustomReward>, PlatformError> {
        let token = self.user_token().await?;
        let ids = Collection::from(&[][..] as &[RewardId]);
        self.helix()
            .get_custom_rewards(self.config.broadcaster_id.clone(), false, &ids, &token)
            .await
            .map_err(|e| PlatformError::TwitchApi(e.to_string()))
    }

    pub async fn find_user_by_login(&self, login: &str) -> Result<Option<User>, PlatformError> {
        let token = self.user_token().await?;
        let helix = self.helix();
        let logins = vec![types::UserName::from(login)].into();
        let mut stream = helix.get_users_from_logins(&logins, &token);
        stream
            .next()
            .await
            .transpose()
            .map_err(|e| PlatformError::TwitchApi(e.to_string()))
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
}

#[cfg(test)]
#[path = "twitch_auth.test.rs"]
mod tests;
