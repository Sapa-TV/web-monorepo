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
#[path = "inmemory_platform.test.rs"]
mod tests;
