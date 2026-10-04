use std::sync::nonpoison::Mutex;

use crate::config::repository::ConfigRepository;
use crate::config::runtime::RuntimeConfig;
use crate::error::RepositoryError;

#[non_exhaustive]
pub struct InMemoryConfigRepository {
    config: Mutex<Option<RuntimeConfig>>,
}

impl InMemoryConfigRepository {
    pub fn new() -> Self {
        Self {
            config: Mutex::new(None),
        }
    }
}

impl Default for InMemoryConfigRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigRepository for InMemoryConfigRepository {
    async fn load(&self) -> Result<Option<RuntimeConfig>, RepositoryError> {
        Ok(self.config.lock().clone())
    }

    async fn save(&self, config: &RuntimeConfig) -> Result<(), RepositoryError> {
        *self.config.lock() = Some(config.clone());
        Ok(())
    }
}

#[cfg(test)]
#[path = "inmemory_config.test.rs"]
mod tests;
