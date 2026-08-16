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
}

impl Platform {
    pub fn new(id: PlatformId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
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
mod tests {
    use super::*;

    #[test]
    fn platform_serde_roundtrip() {
        let platform = Platform::from_id(PlatformId::YOUTUBE);
        let json = serde_json::to_value(&platform).unwrap();
        assert_eq!(json, serde_json::json!({"id": 2, "name": "youtube"}));
        let back: Platform = serde_json::from_value(json).unwrap();
        assert_eq!(back, platform);
    }
}

#[cfg(test)]
mod credential_service_tests {
    use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;

    use super::*;

    fn test_service() -> PlatformCredentialService<InMemoryPlatformCredentialRepository> {
        PlatformCredentialService::new(Arc::new(InMemoryPlatformCredentialRepository::new()))
    }

    #[tokio::test]
    async fn initial_revision_is_zero() {
        let service = test_service();
        assert_eq!(*service.subscribe_lifecycle().borrow(), 0);
    }

    #[tokio::test]
    async fn save_credential_persists_and_bumps() {
        let service = test_service();
        let mut rx = service.subscribe_lifecycle();

        service
            .save_credential(PlatformId::TWITCH, "token")
            .await
            .unwrap();

        assert_eq!(
            service
                .load_credential(PlatformId::TWITCH)
                .await
                .unwrap()
                .as_deref(),
            Some("token")
        );
        assert!(rx.has_changed().unwrap());
        rx.changed().await.unwrap();
        assert_eq!(rx.borrow_and_update().clone(), 1);
    }

    #[tokio::test]
    async fn clear_credential_bumps() {
        let service = test_service();
        service
            .save_credential(PlatformId::TWITCH, "token")
            .await
            .unwrap();
        let mut rx = service.subscribe_lifecycle();

        service.clear_credential(PlatformId::TWITCH).await.unwrap();

        assert_eq!(
            service.load_credential(PlatformId::TWITCH).await.unwrap(),
            None
        );
        assert!(rx.has_changed().unwrap());
        rx.changed().await.unwrap();
        assert_eq!(rx.borrow_and_update().clone(), 2);
    }

    #[tokio::test]
    async fn save_rotated_does_not_bump() {
        let service = test_service();
        let rx = service.subscribe_lifecycle();

        service
            .save_rotated(PlatformId::TWITCH, "rotated")
            .await
            .unwrap();

        assert_eq!(
            service
                .load_credential(PlatformId::TWITCH)
                .await
                .unwrap()
                .as_deref(),
            Some("rotated")
        );
        assert_eq!(*rx.borrow(), 0);
        assert!(
            !rx.has_changed().unwrap(),
            "rotation must not notify lifecycle"
        );
    }

    #[tokio::test]
    async fn lifecycle_is_revision_not_payload() {
        let service = test_service();
        let mut rx = service.subscribe_lifecycle();
        service
            .save_credential(PlatformId::TWITCH, "first")
            .await
            .unwrap();
        service
            .save_credential(PlatformId::TWITCH, "second")
            .await
            .unwrap();
        service
            .save_credential(PlatformId::YOUTUBE, "yt")
            .await
            .unwrap();

        rx.changed().await.unwrap();
        assert_eq!(rx.borrow_and_update().clone(), 3);
        assert_eq!(service.revision(), 3);
    }

    #[tokio::test]
    async fn revision_is_monotonic_under_concurrency() {
        let service = Arc::new(test_service());
        let mut handles = Vec::new();
        for i in 0..10_u64 {
            let service = Arc::clone(&service);
            handles.push(tokio::spawn(async move {
                service
                    .save_credential(PlatformId::TWITCH, &format!("tok-{i}"))
                    .await
                    .unwrap();
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
        assert_eq!(service.revision(), 10, "no revision lost under concurrency");
        assert!(
            service
                .load_credential(PlatformId::TWITCH)
                .await
                .unwrap()
                .is_some()
        );
    }
}
