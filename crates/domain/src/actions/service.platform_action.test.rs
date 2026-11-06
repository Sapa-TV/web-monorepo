use std::sync::Arc;
use std::sync::nonpoison::Mutex;

use super::*;

struct SpyExecutor {
    calls: Arc<Mutex<Vec<String>>>,
}

impl Clone for SpyExecutor {
    fn clone(&self) -> Self {
        Self {
            calls: Arc::clone(&self.calls),
        }
    }
}

impl SpyExecutor {
    fn new() -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn texts(&self) -> Vec<String> {
        self.calls.lock().clone()
    }
}

impl PlatformActionExecutor for SpyExecutor {
    fn platform(&self) -> PlatformId {
        PlatformId::TWITCH
    }

    async fn send_chat_message(&self, _ctx: &ActionContext, text: &str) -> Result<(), ActionError> {
        self.calls.lock().push(text.to_string());
        Ok(())
    }
}

fn action_ctx() -> ActionContext {
    ActionContext::new("e-1".to_string(), "42".to_string(), "viewer".to_string())
}

#[tokio::test]
async fn empty_service_reports_unsupported() {
    let service = PlatformActionService::<SpyExecutor>::new();
    let err = service
        .send_chat_message(PlatformId::TWITCH, &action_ctx(), "hi")
        .await
        .unwrap_err();
    assert!(matches!(err, ActionError::Unsupported));
}

#[tokio::test]
async fn non_twitch_platform_is_unsupported_even_with_executor() {
    let service: PlatformActionService<SpyExecutor> =
        PlatformActionService::new().with_twitch(SpyExecutor::new());
    let err = service
        .send_chat_message(PlatformId::YOUTUBE, &action_ctx(), "hi")
        .await
        .unwrap_err();
    assert!(matches!(err, ActionError::Unsupported));
}

#[tokio::test]
async fn registered_executor_receives_text() {
    let spy = SpyExecutor::new();
    let service: PlatformActionService<SpyExecutor> =
        PlatformActionService::new().with_twitch(spy.clone());

    service
        .send_chat_message(PlatformId::TWITCH, &action_ctx(), "привет!")
        .await
        .unwrap();

    assert_eq!(spy.texts(), vec!["привет!".to_string()]);
}
