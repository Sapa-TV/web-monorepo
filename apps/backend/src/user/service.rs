use std::sync::Arc;
use std::sync::OnceLock;

use crate::error::UserServiceError;
use crate::platform::{Platform, PlatformRepository};
use crate::user::repository::UserRepository;
use crate::user::{ResolvedUserPlatform, User, UserId, UserPlatform, UserView};

pub struct UserService<U, P>
where
    U: UserRepository,
    P: PlatformRepository,
{
    user_repo: Arc<U>,
    platform_repo: Arc<P>,
    guest_user_id: OnceLock<UserId>,
}

impl<U, P> UserService<U, P>
where
    U: UserRepository,
    P: PlatformRepository,
{
    pub fn new(user_repo: Arc<U>, platform_repo: Arc<P>) -> Self {
        Self {
            user_repo,
            platform_repo,
            guest_user_id: OnceLock::new(),
        }
    }

    async fn resolve_platform(&self, name: &str) -> Result<Platform, UserServiceError> {
        self.platform_repo
            .find_by_name(name)
            .await?
            .ok_or_else(|| UserServiceError::UnknownPlatform(name.to_string()))
    }

    pub async fn create(&self, display_name: &str) -> Result<User, UserServiceError> {
        Ok(self.user_repo.create(display_name).await?)
    }

    pub async fn find_by_platform(
        &self,
        platform_name: &str,
        platform_user_id: &str,
    ) -> Result<Option<User>, UserServiceError> {
        let platform = self.resolve_platform(platform_name).await?;
        Ok(self
            .user_repo
            .find_by_platform(platform.id, platform_user_id)
            .await?)
    }

    pub async fn get_user(&self, user_id: UserId) -> Result<Option<User>, UserServiceError> {
        Ok(self.user_repo.get_by_id(user_id).await?)
    }

    pub async fn get_platforms(
        &self,
        user_id: UserId,
    ) -> Result<Vec<UserPlatform>, UserServiceError> {
        Ok(self.user_repo.get_platforms(user_id).await?)
    }

    pub async fn build_user(&self, user_id: UserId) -> Result<Option<UserView>, UserServiceError> {
        let Some(user) = self.user_repo.get_by_id(user_id).await? else {
            return Ok(None);
        };
        let user_platforms = self.user_repo.get_platforms(user_id).await?;
        let platforms = self.resolve_user_platforms(user_platforms).await?;
        Ok(Some(UserView { user, platforms }))
    }

    pub async fn resolve_user_platforms(
        &self,
        user_platforms: Vec<UserPlatform>,
    ) -> Result<Vec<ResolvedUserPlatform>, UserServiceError> {
        let all_platforms = self.platform_repo.load_all().await?;
        Ok(user_platforms
            .into_iter()
            .map(|up| {
                let platform_name = all_platforms
                    .iter()
                    .find(|p| p.id == up.platform_id)
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                ResolvedUserPlatform {
                    id: up.id,
                    platform_name,
                    platform_user_id: up.platform_user_id,
                    platform_username: up.platform_username,
                }
            })
            .collect())
    }

    pub async fn update_user(
        &self,
        user_id: UserId,
        display_name: &str,
    ) -> Result<(), UserServiceError> {
        let updated = self
            .user_repo
            .update_display_name(user_id, display_name)
            .await?;
        if updated.is_none() {
            return Err(UserServiceError::UserNotFound);
        }
        Ok(())
    }

    pub async fn delete_user(&self, user_id: UserId) -> Result<(), UserServiceError> {
        let deleted = self.user_repo.delete_user(user_id).await?;
        if !deleted {
            return Err(UserServiceError::UserNotFound);
        }
        Ok(())
    }

    pub async fn link_platform(
        &self,
        user_id: UserId,
        platform_name: &str,
        platform_user_id: &str,
        platform_username: &str,
    ) -> Result<(), UserServiceError> {
        if self.user_repo.get_by_id(user_id).await?.is_none() {
            return Err(UserServiceError::UserNotFound);
        }
        let platform = self.resolve_platform(platform_name).await?;
        self.user_repo
            .link_platform(user_id, platform.id, platform_user_id, platform_username)
            .await?;
        Ok(())
    }

    pub async fn update_platform_username(
        &self,
        user_id: UserId,
        platform_name: &str,
        platform_username: &str,
    ) -> Result<(), UserServiceError> {
        let platform = self.resolve_platform(platform_name).await?;
        let updated = self
            .user_repo
            .update_platform_username(user_id, platform.id, platform_username)
            .await?;
        if updated.is_none() {
            return Err(UserServiceError::PlatformLinkNotFound);
        }
        Ok(())
    }

    pub async fn delete_platform(
        &self,
        user_id: UserId,
        platform_name: &str,
    ) -> Result<(), UserServiceError> {
        let platform = self.resolve_platform(platform_name).await?;
        let deleted = self.user_repo.delete_platform(user_id, platform.id).await?;
        if !deleted {
            return Err(UserServiceError::PlatformLinkNotFound);
        }
        Ok(())
    }

    pub async fn list_platforms(&self) -> Result<Vec<Platform>, UserServiceError> {
        Ok(self.platform_repo.load_all().await?)
    }

    pub async fn guest_user_id(&self) -> Result<UserId, UserServiceError> {
        if let Some(id) = self.guest_user_id.get() {
            return Ok(*id);
        }
        let user = self.user_repo.create("guest").await?;
        Ok(*self.guest_user_id.get_or_init(|| user.id))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::db::inmemory_platform::InMemoryPlatformRepository;
    use crate::db::inmemory_user::InMemoryUserRepository;
    use crate::error::UserServiceError;
    use crate::platform::PlatformId;

    use super::*;

    type TestService = UserService<InMemoryUserRepository, InMemoryPlatformRepository>;

    async fn test_service() -> TestService {
        UserService::new(
            Arc::new(InMemoryUserRepository::new()),
            Arc::new(InMemoryPlatformRepository::new_seeded()),
        )
    }

    async fn create_user(svc: &TestService, name: &str) -> User {
        svc.create(name).await.unwrap()
    }

    #[tokio::test]
    async fn guest_user_id_is_cached() {
        let svc = test_service().await;
        let first = svc.guest_user_id().await.unwrap();
        let second = svc.guest_user_id().await.unwrap();
        assert_eq!(first, second);
        let user = svc.get_user(first).await.unwrap().unwrap();
        assert_eq!(user.display_name, "guest");
    }

    #[tokio::test]
    async fn find_by_platform_unknown_platform() {
        let svc = test_service().await;
        let err = svc.find_by_platform("unknown", "123").await.unwrap_err();
        assert!(matches!(err, UserServiceError::UnknownPlatform(_)));
    }

    #[tokio::test]
    async fn find_by_platform_not_found() {
        let svc = test_service().await;
        let result = svc.find_by_platform("twitch", "123").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn find_by_platform_found() {
        let svc = test_service().await;
        let user = create_user(&svc, "Viewer").await;
        svc.link_platform(user.id, "twitch", "123", "tw_user")
            .await
            .unwrap();
        let found = svc
            .find_by_platform("twitch", "123")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.id, user.id);
    }

    #[tokio::test]
    async fn link_platform_nonexistent_user() {
        let svc = test_service().await;
        let err = svc
            .link_platform(UserId::new(999), "twitch", "123", "u")
            .await
            .unwrap_err();
        assert!(matches!(err, UserServiceError::UserNotFound));
    }

    #[tokio::test]
    async fn link_platform_unknown_platform() {
        let svc = test_service().await;
        let user = create_user(&svc, "Viewer").await;
        let err = svc
            .link_platform(user.id, "unknown", "123", "u")
            .await
            .unwrap_err();
        assert!(matches!(err, UserServiceError::UnknownPlatform(_)));
    }

    #[tokio::test]
    async fn update_user_nonexistent() {
        let svc = test_service().await;
        let err = svc.update_user(UserId::new(999), "Name").await.unwrap_err();
        assert!(matches!(err, UserServiceError::UserNotFound));
    }

    #[tokio::test]
    async fn delete_user_nonexistent() {
        let svc = test_service().await;
        let err = svc.delete_user(UserId::new(999)).await.unwrap_err();
        assert!(matches!(err, UserServiceError::UserNotFound));
    }

    #[tokio::test]
    async fn update_platform_username_missing_link() {
        let svc = test_service().await;
        let user = create_user(&svc, "Viewer").await;
        let err = svc
            .update_platform_username(user.id, "twitch", "name")
            .await
            .unwrap_err();
        assert!(matches!(err, UserServiceError::PlatformLinkNotFound));
    }

    #[tokio::test]
    async fn delete_platform_missing_link() {
        let svc = test_service().await;
        let user = create_user(&svc, "Viewer").await;
        let err = svc.delete_platform(user.id, "twitch").await.unwrap_err();
        assert!(matches!(err, UserServiceError::PlatformLinkNotFound));
    }

    #[tokio::test]
    async fn update_platform_username_unknown_platform() {
        let svc = test_service().await;
        let user = create_user(&svc, "Viewer").await;
        let err = svc
            .update_platform_username(user.id, "unknown", "name")
            .await
            .unwrap_err();
        assert!(matches!(err, UserServiceError::UnknownPlatform(_)));
    }

    #[tokio::test]
    async fn update_user_happy_path() {
        let svc = test_service().await;
        let user = create_user(&svc, "Old").await;
        svc.update_user(user.id, "New").await.unwrap();
        let fetched = svc.get_user(user.id).await.unwrap().unwrap();
        assert_eq!(fetched.display_name, "New");
    }

    #[tokio::test]
    async fn delete_user_happy_path() {
        let svc = test_service().await;
        let user = create_user(&svc, "Viewer").await;
        svc.delete_user(user.id).await.unwrap();
        assert!(svc.get_user(user.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn build_user_resolves_platform_names() {
        let svc = test_service().await;
        let user = create_user(&svc, "Viewer").await;
        svc.link_platform(user.id, "twitch", "123", "tw_user")
            .await
            .unwrap();

        let view = svc.build_user(user.id).await.unwrap().unwrap();
        assert_eq!(view.user.display_name, "Viewer");
        assert_eq!(view.platforms.len(), 1);
        assert_eq!(view.platforms[0].platform_name, "twitch");
        assert_eq!(view.platforms[0].platform_user_id, "123");
        assert_eq!(view.platforms[0].platform_username, "tw_user");
    }

    #[tokio::test]
    async fn build_user_nonexistent() {
        let svc = test_service().await;
        assert!(svc.build_user(UserId::new(999)).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn build_user_unknown_platform_falls_back_to_empty_name() {
        let svc = test_service().await;
        let user = create_user(&svc, "Viewer").await;
        svc.link_platform(user.id, "twitch", "123", "tw_user")
            .await
            .unwrap();
        let user_platforms = svc.get_platforms(user.id).await.unwrap();
        let mut broken = user_platforms;
        broken[0].platform_id = PlatformId::new(999);

        let resolved = svc.resolve_user_platforms(broken).await.unwrap();
        assert_eq!(resolved[0].platform_name, "");
    }
}
