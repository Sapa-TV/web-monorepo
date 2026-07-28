use std::future::Future;

use crate::error::RepositoryError;
use crate::platform::PlatformId;
use crate::user::{User, UserId, UserPlatform};

pub trait UserRepository: Send + Sync {
    fn create(
        &self,
        display_name: &str,
    ) -> impl Future<Output = Result<User, RepositoryError>> + Send;

    fn find_by_platform(
        &self,
        platform_id: PlatformId,
        platform_user_id: &str,
    ) -> impl Future<Output = Result<Option<User>, RepositoryError>> + Send;

    fn get_by_id(
        &self,
        id: UserId,
    ) -> impl Future<Output = Result<Option<User>, RepositoryError>> + Send;

    fn get_platforms(
        &self,
        user_id: UserId,
    ) -> impl Future<Output = Result<Vec<UserPlatform>, RepositoryError>> + Send;

    fn link_platform(
        &self,
        user_id: UserId,
        platform_id: PlatformId,
        platform_user_id: &str,
        platform_username: &str,
    ) -> impl Future<Output = Result<UserPlatform, RepositoryError>> + Send;

    fn update_display_name(
        &self,
        user_id: UserId,
        display_name: &str,
    ) -> impl Future<Output = Result<Option<User>, RepositoryError>> + Send;

    fn update_platform_username(
        &self,
        user_id: UserId,
        platform_id: PlatformId,
        platform_username: &str,
    ) -> impl Future<Output = Result<Option<UserPlatform>, RepositoryError>> + Send;

    fn delete_platform(
        &self,
        user_id: UserId,
        platform_id: PlatformId,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;

    fn delete_user(
        &self,
        id: UserId,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
}
