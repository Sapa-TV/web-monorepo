use std::future::Future;

use crate::config::runtime::RuntimeConfig;
use crate::error::RepositoryError;

pub trait ConfigRepository: Send + Sync {
    fn load(&self) -> impl Future<Output = Result<Option<RuntimeConfig>, RepositoryError>> + Send;

    fn save(
        &self,
        config: &RuntimeConfig,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}
