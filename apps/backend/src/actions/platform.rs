use std::future::Future;

use crate::error::platform_action::ActionError;
use crate::platform::PlatformId;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ActionContext {
    pub event_id: String,
    pub user_id: String,
    pub user_name: String,
    pub channel_id: String,
}

pub trait PlatformActionExecutor: Send + Sync {
    fn platform(&self) -> PlatformId;

    fn send_chat_message(
        &self,
        _ctx: &ActionContext,
        _text: &str,
    ) -> impl Future<Output = Result<(), ActionError>> + Send {
        let platform = self.platform();
        async move {
            tracing::warn!(
                platform = platform.name(),
                action = "send_chat_message",
                "capability not supported"
            );
            Err(ActionError::Unsupported)
        }
    }
}
