use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::watch;

use crate::error::RepositoryError;

pub use model::platform::{Platform, PlatformCredentialRepository, PlatformId, PlatformRepository};

#[non_exhaustive]
pub struct PlatformCredentialService<C>
where
    C: PlatformCredentialRepository,
{
    repo: Arc<C>,
    revision: AtomicU64,
    lifecycle: watch::Sender<u64>,
}

impl<C> PlatformCredentialService<C>
where
    C: PlatformCredentialRepository,
{
    pub fn new(repo: Arc<C>) -> Self {
        Self {
            repo,
            revision: AtomicU64::new(0),
            lifecycle: watch::channel(0).0,
        }
    }

    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

    pub async fn load_credential(
        &self,
        platform: PlatformId,
    ) -> Result<Option<String>, RepositoryError> {
        self.repo.load_credential(platform).await
    }

    pub async fn save_credential(
        &self,
        platform: PlatformId,
        credential: &str,
    ) -> Result<(), RepositoryError> {
        self.repo.save_credential(platform, credential).await?;
        self.bump();
        Ok(())
    }

    pub async fn save_rotated(
        &self,
        platform: PlatformId,
        credential: &str,
    ) -> Result<(), RepositoryError> {
        self.repo.save_credential(platform, credential).await
    }

    pub async fn clear_credential(&self, platform: PlatformId) -> Result<(), RepositoryError> {
        self.repo.clear_credential(platform).await?;
        self.bump();
        Ok(())
    }

    pub fn subscribe_lifecycle(&self) -> watch::Receiver<u64> {
        self.lifecycle.subscribe()
    }

    fn bump(&self) {
        let next = self.revision.fetch_add(1, Ordering::Relaxed) + 1;
        self.lifecycle.send_replace(next);
    }
}

#[cfg(test)]
#[path = "platform.credential_service.test.rs"]
mod credential_service_tests;
