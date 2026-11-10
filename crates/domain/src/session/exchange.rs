use std::future::Future;
use std::sync::Arc;

use model::platform::PlatformId;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExchangedToken {
    pub platform_id: PlatformId,
    pub user_id: String,
    pub user_name: Option<String>,
}

impl ExchangedToken {
    pub fn new(platform_id: PlatformId, user_id: String, user_name: Option<String>) -> Self {
        Self {
            platform_id,
            user_id,
            user_name,
        }
    }
}

pub trait LoginExchange: Send + Sync {
    fn complete_login(
        &self,
        code: &str,
        auth_state: &str,
    ) -> impl Future<Output = Result<ExchangedToken, String>> + Send;
}

impl<T: LoginExchange + ?Sized> LoginExchange for Arc<T> {
    fn complete_login(
        &self,
        code: &str,
        auth_state: &str,
    ) -> impl Future<Output = Result<ExchangedToken, String>> + Send {
        (**self).complete_login(code, auth_state)
    }
}
