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
#[path = "twitch_executor.test.rs"]
mod tests;
