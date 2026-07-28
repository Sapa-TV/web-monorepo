use std::sync::nonpoison::Mutex;

use crate::error::RepositoryError;
use crate::platform::{Platform, PlatformId, PlatformRepository};

const SEEDED: &[(u32, &str)] = &[
    (1, "twitch"),
    (2, "youtube"),
    (3, "vk_video_live"),
];

pub struct InMemoryPlatformRepository {
    platforms: Mutex<Vec<Platform>>,
}

impl InMemoryPlatformRepository {
    pub fn new_seeded() -> Self {
        let platforms = SEEDED
            .iter()
            .map(|&(id, name)| Platform {
                id: PlatformId::new(id),
                name: name.to_string(),
            })
            .collect();
        Self {
            platforms: Mutex::new(platforms),
        }
    }
}

impl PlatformRepository for InMemoryPlatformRepository {
    async fn find_by_name(&self, name: &str) -> Result<Option<Platform>, RepositoryError> {
        let platforms = self.platforms.lock();
        Ok(platforms.iter().find(|p| p.name == name).cloned())
    }

    async fn find_by_id(&self, id: PlatformId) -> Result<Option<Platform>, RepositoryError> {
        let platforms = self.platforms.lock();
        Ok(platforms.iter().find(|p| p.id == id).cloned())
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
    async fn find_by_id_found() {
        let repo = InMemoryPlatformRepository::new_seeded();
        let result = repo.find_by_id(PlatformId::new(1)).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, PlatformId::new(1));
    }

    #[tokio::test]
    async fn find_by_id_not_found() {
        let repo = InMemoryPlatformRepository::new_seeded();
        let result = repo.find_by_id(PlatformId::new(999)).await.unwrap();
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
