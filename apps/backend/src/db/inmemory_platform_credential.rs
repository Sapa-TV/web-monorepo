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
mod tests {
    use super::*;

    #[tokio::test]
    async fn roundtrip() {
        let repo = InMemoryPlatformCredentialRepository::new();
        assert_eq!(
            repo.load_credential(PlatformId::TWITCH).await.unwrap(),
            None
        );

        repo.save_credential(PlatformId::TWITCH, "refresh_token_1")
            .await
            .unwrap();
        assert_eq!(
            repo.load_credential(PlatformId::TWITCH)
                .await
                .unwrap()
                .as_deref(),
            Some("refresh_token_1")
        );
    }

    #[tokio::test]
    async fn clear_removes_credential() {
        let repo = InMemoryPlatformCredentialRepository::new();
        repo.save_credential(PlatformId::VK_VIDEO_LIVE, "token")
            .await
            .unwrap();
        repo.clear_credential(PlatformId::VK_VIDEO_LIVE)
            .await
            .unwrap();
        assert_eq!(
            repo.load_credential(PlatformId::VK_VIDEO_LIVE)
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn credentials_are_per_platform() {
        let repo = InMemoryPlatformCredentialRepository::new();
        repo.save_credential(PlatformId::TWITCH, "twitch_token")
            .await
            .unwrap();
        assert_eq!(
            repo.load_credential(PlatformId::TWITCH)
                .await
                .unwrap()
                .as_deref(),
            Some("twitch_token")
        );
        assert_eq!(
            repo.load_credential(PlatformId::YOUTUBE).await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn save_trims_credential() {
        let repo = InMemoryPlatformCredentialRepository::new();
        repo.save_credential(PlatformId::TWITCH, "  tok  ")
            .await
            .unwrap();
        assert_eq!(
            repo.load_credential(PlatformId::TWITCH)
                .await
                .unwrap()
                .as_deref(),
            Some("tok")
        );
    }
}
