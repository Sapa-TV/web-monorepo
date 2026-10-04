use std::fmt::{self, Display};
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use utoipa::ToSchema;

use crate::error::RepositoryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct PlatformId(u32);

impl PlatformId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub const TWITCH: PlatformId = PlatformId::new(1);
    pub const YOUTUBE: PlatformId = PlatformId::new(2);
    pub const VK_VIDEO_LIVE: PlatformId = PlatformId::new(3);

    pub const fn name(self) -> &'static str {
        match self.0 {
            1 => "twitch",
            2 => "youtube",
            3 => "vk_video_live",
            _ => "unknown",
        }
    }
}

impl Display for PlatformId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Platform {
    pub id: PlatformId,
    pub name: String,
    #[serde(skip)]
    _sealed: (),
}

impl Platform {
    pub fn new(id: PlatformId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            _sealed: (),
        }
    }

    pub fn from_id(id: PlatformId) -> Self {
        Self::new(id, id.name())
    }

    pub fn as_name(&self) -> &'static str {
        self.id.name()
    }
}

impl Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_name())
    }
}

pub trait PlatformCredentialRepository: Send + Sync {
    fn load_credential(
        &self,
        platform: PlatformId,
    ) -> impl Future<Output = Result<Option<String>, RepositoryError>> + Send;
    fn save_credential(
        &self,
        platform: PlatformId,
        credential: &str,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn clear_credential(
        &self,
        platform: PlatformId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

pub trait PlatformRepository: Send + Sync {
    fn find_by_name(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Option<Platform>, RepositoryError>> + Send;
    fn load_all(&self) -> impl Future<Output = Result<Vec<Platform>, RepositoryError>> + Send;
}

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
#[path = "platform.test.rs"]
mod tests;

#[cfg(test)]
#[path = "platform.credential_service.test.rs"]
mod credential_service_tests;
