use std::collections::HashMap;
use std::sync::nonpoison::Mutex;

use crate::error::RepositoryError;
use crate::platform::{PlatformCredentialRepository, PlatformId};

#[non_exhaustive]
pub struct InMemoryPlatformCredentialRepository {
    credentials: Mutex<HashMap<PlatformId, String>>,
}

impl InMemoryPlatformCredentialRepository {
    pub fn new() -> Self {
        Self {
            credentials: Mutex::new(HashMap::new()),
        }
    }

    pub fn seeded(credentials: impl IntoIterator<Item = (PlatformId, impl Into<String>)>) -> Self {
        let credentials = credentials
            .into_iter()
            .map(|(platform, credential)| (platform, credential.into()))
            .collect();
        Self {
            credentials: Mutex::new(credentials),
        }
    }
}

impl Default for InMemoryPlatformCredentialRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformCredentialRepository for InMemoryPlatformCredentialRepository {
    async fn load_credential(
        &self,
        platform: PlatformId,
    ) -> Result<Option<String>, RepositoryError> {
        Ok(self.credentials.lock().get(&platform).cloned())
    }

    async fn save_credential(
        &self,
        platform: PlatformId,
        credential: &str,
    ) -> Result<(), RepositoryError> {
        self.credentials
            .lock()
            .insert(platform, credential.trim().to_string());
        Ok(())
    }

    async fn clear_credential(&self, platform: PlatformId) -> Result<(), RepositoryError> {
        self.credentials.lock().remove(&platform);
        Ok(())
    }
}

#[cfg(test)]
#[path = "inmemory_platform_credential.test.rs"]
mod tests;
