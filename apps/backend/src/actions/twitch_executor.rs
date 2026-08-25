use std::sync::Arc;

use twitch_oauth2::TwitchToken;

use crate::actions::platform::ActionContext;
use crate::actions::platform::PlatformActionExecutor;
use crate::config::TwitchConfig;
use crate::error::platform_action::ActionError;
use crate::ingress::twitch_auth::TwitchAuthService;
use crate::platform::PlatformCredentialRepository;
use crate::platform::PlatformId;

pub struct TwitchActionExecutor<C>
where
    C: PlatformCredentialRepository + Send + Sync,
{
    channel_id: String,
    auth: Arc<TwitchAuthService<C>>,
}

impl<C> TwitchActionExecutor<C>
where
    C: PlatformCredentialRepository + Send + Sync,
{
    pub fn new(config: Arc<TwitchConfig>, auth: Arc<TwitchAuthService<C>>) -> Self {
        Self {
            channel_id: config.broadcaster_id.clone(),
            auth,
        }
    }
}

impl<C> PlatformActionExecutor for TwitchActionExecutor<C>
where
    C: PlatformCredentialRepository + Send + Sync,
{
    fn platform(&self) -> PlatformId {
        PlatformId::TWITCH
    }

    async fn send_chat_message(&self, _ctx: &ActionContext, text: &str) -> Result<(), ActionError> {
        let token = self
            .auth
            .user_token()
            .await
            .map_err(|e| ActionError::Api(e.to_string()))?;
        let sender_id = token
            .user_id()
            .ok_or_else(|| ActionError::Api("token has no user_id".to_string()))?;
        let helix = self.auth.helix();
        helix
            .send_chat_message(&self.channel_id, sender_id, text, &token)
            .await
            .map_err(|e| ActionError::Api(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
    use crate::platform::PlatformCredentialService;

    fn executor_without_credentials() -> TwitchActionExecutor<InMemoryPlatformCredentialRepository>
    {
        let config = Arc::new(TwitchConfig {
            client_id: "cid".to_string(),
            client_secret: "cs".to_string(),
            broadcaster_id: "bc".to_string(),
            redirect_uri: "https://localhost/cb".to_string(),
            credentials_redirect_uri: "https://localhost/creds/cb".to_string(),
            csrf_ttl_secs: 600,
        });
        let credentials = Arc::new(PlatformCredentialService::new(Arc::new(
            InMemoryPlatformCredentialRepository::new(),
        )));
        let auth = Arc::new(TwitchAuthService::new(Arc::clone(&config), credentials));
        TwitchActionExecutor::new(config, auth)
    }

    #[tokio::test]
    async fn platform_is_twitch() {
        assert_eq!(
            executor_without_credentials().platform(),
            PlatformId::TWITCH
        );
    }

    #[tokio::test]
    async fn missing_token_maps_to_api_error() {
        let executor = executor_without_credentials();
        let ctx = ActionContext::new("e".to_string(), "1".to_string(), "u".to_string());

        let err = executor
            .send_chat_message(&ctx, "hello")
            .await
            .expect_err("no credentials stored");

        assert!(matches!(err, ActionError::Api(_)));
    }
}
