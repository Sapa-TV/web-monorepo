use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::nonpoison::Mutex;

use chrono::Utc;

use crate::error::RepositoryError;
use crate::platform::PlatformId;
use crate::user::repository::UserRepository;
use crate::user::{User, UserId, UserPlatform, UserPlatformId};

pub struct InMemoryUserRepository {
    users: Mutex<Vec<User>>,
    user_platforms: Mutex<Vec<UserPlatform>>,
    next_user_id: AtomicU32,
    next_platform_id: AtomicU32,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self {
            users: Mutex::new(Vec::new()),
            user_platforms: Mutex::new(Vec::new()),
            next_user_id: AtomicU32::new(1),
            next_platform_id: AtomicU32::new(1),
        }
    }
}

impl UserRepository for InMemoryUserRepository {
    async fn create(&self, display_name: &str) -> Result<User, RepositoryError> {
        let id = self.next_user_id.fetch_add(1, Ordering::Relaxed);
        let now = Utc::now().naive_utc();
        let user = User {
            id: UserId::new(id),
            display_name: display_name.to_string(),
            created_at: now,
            updated_at: now,
        };
        self.users.lock().push(user.clone());
        Ok(user)
    }

    async fn find_by_platform(
        &self,
        platform_id: PlatformId,
        platform_user_id: &str,
    ) -> Result<Option<User>, RepositoryError> {
        let user_platforms = self.user_platforms.lock();
        if let Some(up) = user_platforms
            .iter()
            .find(|up| up.platform_id == platform_id && up.platform_user_id == platform_user_id)
        {
            let users = self.users.lock();
            Ok(users.iter().find(|u| u.id == up.user_id).cloned())
        } else {
            Ok(None)
        }
    }

    async fn get_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError> {
        let users = self.users.lock();
        Ok(users.iter().find(|u| u.id == id).cloned())
    }

    async fn get_platforms(&self, user_id: UserId) -> Result<Vec<UserPlatform>, RepositoryError> {
        let user_platforms = self.user_platforms.lock();
        Ok(user_platforms
            .iter()
            .filter(|up| up.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn link_platform(
        &self,
        user_id: UserId,
        platform_id: PlatformId,
        platform_user_id: &str,
        platform_username: &str,
    ) -> Result<UserPlatform, RepositoryError> {
        let exists = {
            let user_platforms = self.user_platforms.lock();
            user_platforms
                .iter()
                .any(|up| up.platform_id == platform_id && up.platform_user_id == platform_user_id)
        };
        if exists {
            return Err(RepositoryError::Conflict(
                "platform_user_id already linked to another user".to_string(),
            ));
        }

        let id = self.next_platform_id.fetch_add(1, Ordering::Relaxed);
        let user_platform = UserPlatform {
            id: UserPlatformId::new(id),
            user_id,
            platform_id,
            platform_user_id: platform_user_id.to_string(),
            platform_username: platform_username.to_string(),
        };

        {
            let mut users = self.users.lock();
            if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
                user.updated_at = Utc::now().naive_utc();
            }
        }

        self.user_platforms.lock().push(user_platform.clone());
        Ok(user_platform)
    }

    async fn update_display_name(
        &self,
        user_id: UserId,
        display_name: &str,
    ) -> Result<Option<User>, RepositoryError> {
        let mut users = self.users.lock();
        let user = users.iter_mut().find(|u| u.id == user_id);
        match user {
            Some(user) => {
                user.display_name = display_name.to_string();
                user.updated_at = Utc::now().naive_utc();
                Ok(Some(user.clone()))
            }
            None => Ok(None),
        }
    }

    async fn update_platform_username(
        &self,
        user_id: UserId,
        platform_id: PlatformId,
        platform_username: &str,
    ) -> Result<Option<UserPlatform>, RepositoryError> {
        let mut user_platforms = self.user_platforms.lock();
        let up = user_platforms
            .iter_mut()
            .find(|up| up.user_id == user_id && up.platform_id == platform_id);

        match up {
            Some(up) => {
                up.platform_username = platform_username.to_string();
                let result = up.clone();
                drop(user_platforms);

                let mut users = self.users.lock();
                if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
                    user.updated_at = Utc::now().naive_utc();
                }

                Ok(Some(result))
            }
            None => Ok(None),
        }
    }

    async fn delete_platform(
        &self,
        user_id: UserId,
        platform_id: PlatformId,
    ) -> Result<bool, RepositoryError> {
        let mut user_platforms = self.user_platforms.lock();
        let len_before = user_platforms.len();
        user_platforms.retain(|up| !(up.user_id == user_id && up.platform_id == platform_id));
        if user_platforms.len() == len_before {
            return Ok(false);
        }

        drop(user_platforms);

        let mut users = self.users.lock();
        if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
            user.updated_at = Utc::now().naive_utc();
        }

        Ok(true)
    }

    async fn delete_user(&self, id: UserId) -> Result<bool, RepositoryError> {
        {
            let mut users = self.users.lock();
            let len_before = users.len();
            users.retain(|u| u.id != id);
            if users.len() == len_before {
                return Ok(false);
            }
        }

        let mut user_platforms = self.user_platforms.lock();
        user_platforms.retain(|up| up.user_id != id);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use crate::platform::PlatformId;

    use super::*;

    const TWITCH: PlatformId = PlatformId::new(1);
    const YOUTUBE: PlatformId = PlatformId::new(2);

    #[tokio::test]
    async fn create_user() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("Viewer").await.unwrap();
        assert_eq!(user.display_name, "Viewer");
        assert_eq!(user.id.value(), 1);
    }

    #[tokio::test]
    async fn find_by_platform_found() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("Viewer").await.unwrap();
        repo.link_platform(user.id, TWITCH, "123", "viewer_name")
            .await
            .unwrap();

        let found = repo.find_by_platform(TWITCH, "123").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, user.id);
    }

    #[tokio::test]
    async fn find_by_platform_not_found() {
        let repo = InMemoryUserRepository::new();
        let result = repo.find_by_platform(TWITCH, "unknown").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn link_platform_duplicate_global() {
        let repo = InMemoryUserRepository::new();
        let user1 = repo.create("A").await.unwrap();
        let user2 = repo.create("B").await.unwrap();
        repo.link_platform(user1.id, TWITCH, "123", "user_a")
            .await
            .unwrap();

        let err = repo
            .link_platform(user2.id, TWITCH, "123", "user_b")
            .await
            .unwrap_err();
        assert_eq!(
            err,
            RepositoryError::Conflict(
                "platform_user_id already linked to another user".to_string()
            )
        );
    }

    #[tokio::test]
    async fn get_platforms_returns_linked() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("Viewer").await.unwrap();
        repo.link_platform(user.id, TWITCH, "t123", "twitch_user")
            .await
            .unwrap();
        repo.link_platform(user.id, YOUTUBE, "y456", "yt_user")
            .await
            .unwrap();

        let platforms = repo.get_platforms(user.id).await.unwrap();
        assert_eq!(platforms.len(), 2);
    }

    #[tokio::test]
    async fn get_platforms_empty() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("Viewer").await.unwrap();
        let platforms = repo.get_platforms(user.id).await.unwrap();
        assert!(platforms.is_empty());
    }

    #[tokio::test]
    async fn update_display_name() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("OldName").await.unwrap();
        let updated = repo
            .update_display_name(user.id, "NewName")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.display_name, "NewName");

        let fetched = repo.get_by_id(user.id).await.unwrap().unwrap();
        assert_eq!(fetched.display_name, "NewName");
    }

    #[tokio::test]
    async fn update_display_name_nonexistent() {
        let repo = InMemoryUserRepository::new();
        let result = repo
            .update_display_name(UserId::new(999), "Name")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn update_platform_username() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("Viewer").await.unwrap();
        repo.link_platform(user.id, TWITCH, "123", "old_name")
            .await
            .unwrap();

        let updated = repo
            .update_platform_username(user.id, TWITCH, "new_name")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.platform_username, "new_name");

        let platforms = repo.get_platforms(user.id).await.unwrap();
        assert_eq!(platforms[0].platform_username, "new_name");
    }

    #[tokio::test]
    async fn update_platform_username_nonexistent_link() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("Viewer").await.unwrap();
        let result = repo
            .update_platform_username(user.id, TWITCH, "name")
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn get_by_id_found() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("Viewer").await.unwrap();
        let fetched = repo.get_by_id(user.id).await.unwrap().unwrap();
        assert_eq!(fetched.id, user.id);
        assert_eq!(fetched.display_name, "Viewer");
    }

    #[tokio::test]
    async fn get_by_id_not_found() {
        let repo = InMemoryUserRepository::new();
        let result = repo.get_by_id(UserId::new(999)).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn delete_platform_removes_link() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("Viewer").await.unwrap();
        repo.link_platform(user.id, TWITCH, "123", "user")
            .await
            .unwrap();
        repo.link_platform(user.id, YOUTUBE, "456", "yt_user")
            .await
            .unwrap();

        repo.delete_platform(user.id, TWITCH).await.unwrap();
        let platforms = repo.get_platforms(user.id).await.unwrap();
        assert_eq!(platforms.len(), 1);
        assert_eq!(platforms[0].platform_id, YOUTUBE);
    }

    #[tokio::test]
    async fn delete_platform_nonexistent() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("Viewer").await.unwrap();
        let result = repo.delete_platform(user.id, TWITCH).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn delete_user_removes_user_and_platforms() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("Viewer").await.unwrap();
        repo.link_platform(user.id, TWITCH, "123", "user")
            .await
            .unwrap();

        let deleted = repo.delete_user(user.id).await.unwrap();
        assert!(deleted);

        let result = repo.get_by_id(user.id).await.unwrap();
        assert!(result.is_none());

        let platforms = repo.get_platforms(user.id).await.unwrap();
        assert!(platforms.is_empty());
    }

    #[tokio::test]
    async fn delete_user_nonexistent() {
        let repo = InMemoryUserRepository::new();
        let result = repo.delete_user(UserId::new(999)).await.unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn updated_at_changes_on_update() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("Viewer").await.unwrap();
        let original = user.updated_at;

        let updated = repo
            .update_display_name(user.id, "New")
            .await
            .unwrap()
            .unwrap();
        assert!(updated.updated_at > original);
    }

    #[tokio::test]
    async fn updated_at_changes_on_link_platform() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("Viewer").await.unwrap();
        let original = user.updated_at;

        repo.link_platform(user.id, TWITCH, "123", "user")
            .await
            .unwrap();

        let fetched = repo.get_by_id(user.id).await.unwrap().unwrap();
        assert!(fetched.updated_at > original);
    }

    #[tokio::test]
    async fn updated_at_changes_on_delete_platform() {
        let repo = InMemoryUserRepository::new();
        let user = repo.create("Viewer").await.unwrap();
        repo.link_platform(user.id, TWITCH, "123", "user")
            .await
            .unwrap();
        let before_delete = repo.get_by_id(user.id).await.unwrap().unwrap().updated_at;

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        repo.delete_platform(user.id, TWITCH).await.unwrap();

        let fetched = repo.get_by_id(user.id).await.unwrap().unwrap();
        assert!(fetched.updated_at > before_delete);
    }
}
