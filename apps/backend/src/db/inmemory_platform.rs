use std::sync::nonpoison::Mutex;

use crate::error::RepositoryError;
use crate::platform::{Platform, PlatformId, PlatformRepository};

#[non_exhaustive]
pub struct InMemoryPlatformRepository {
    platforms: Mutex<Vec<Platform>>,
}

impl InMemoryPlatformRepository {
    pub fn new_seeded() -> Self {
        let platforms = [
            Platform::from_id(PlatformId::TWITCH),
            Platform::from_id(PlatformId::YOUTUBE),
            Platform::from_id(PlatformId::VK_VIDEO_LIVE),
        ];
        Self {
            platforms: Mutex::new(platforms.to_vec()),
        }
    }
}

impl PlatformRepository for InMemoryPlatformRepository {
    async fn find_by_name(&self, name: &str) -> Result<Option<Platform>, RepositoryError> {
        let platforms = self.platforms.lock();
        Ok(platforms.iter().find(|p| p.name == name).cloned())
    }

    async fn load_all(&self) -> Result<Vec<Platform>, RepositoryError> {
        Ok(self.platforms.lock().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn find_by_name_found() {
        let repo = InMemoryPlatformRepository::new_seeded();
        let result = repo.find_by_name("twitch").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "twitch");
    }

    #[tokio::test]
    async fn find_by_name_not_found() {
        let repo = InMemoryPlatformRepository::new_seeded();
        let result = repo.find_by_name("unknown").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn load_all_returns_seeded() {
        let repo = InMemoryPlatformRepository::new_seeded();
        let all = repo.load_all().await.unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].name, "twitch");
        assert_eq!(all[1].name, "youtube");
        assert_eq!(all[2].name, "vk_video_live");
    }
}
