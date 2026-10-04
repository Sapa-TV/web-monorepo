use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::nonpoison::Mutex;

use chrono::Utc;

use crate::error::RepositoryError;
use crate::platform::PlatformId;
use crate::user::repository::UserRepository;
use crate::user::{User, UserId, UserPlatform, UserPlatformId};

#[non_exhaustive]
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

impl Default for InMemoryUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl UserRepository for InMemoryUserRepository {
    async fn create(&self, display_name: &str) -> Result<User, RepositoryError> {
        let id = self.next_user_id.fetch_add(1, Ordering::Relaxed);
        let now = Utc::now();
        let user = User::new(UserId::new(id), display_name.to_string(), now, now);
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
        let user_platform = UserPlatform::new(
            UserPlatformId::new(id),
            user_id,
            platform_id,
            platform_user_id.to_string(),
            platform_username.to_string(),
        );

        {
            let mut users = self.users.lock();
            if let Some(user) = users.iter_mut().find(|u| u.id == user_id) {
                user.updated_at = Utc::now();
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
                user.updated_at = Utc::now();
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
                    user.updated_at = Utc::now();
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
            user.updated_at = Utc::now();
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
#[path = "inmemory_user.test.rs"]
mod tests;
