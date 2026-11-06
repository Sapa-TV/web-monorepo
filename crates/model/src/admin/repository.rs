use std::future::Future;

use crate::admin::Admin;
use crate::error::RepositoryError;

pub trait AdminRepository: Send + Sync {
    fn create(
        &self,
        twitch_id: &str,
        display_name: Option<&str>,
        is_root: bool,
    ) -> impl Future<Output = Result<Admin, RepositoryError>> + Send;

    fn get_by_twitch_id(
        &self,
        twitch_id: &str,
    ) -> impl Future<Output = Result<Option<Admin>, RepositoryError>> + Send;

    fn list(&self) -> impl Future<Output = Result<Vec<Admin>, RepositoryError>> + Send;

    fn update_display_name(
        &self,
        twitch_id: &str,
        display_name: &str,
    ) -> impl Future<Output = Result<Option<Admin>, RepositoryError>> + Send;

    fn set_root(
        &self,
        twitch_id: &str,
        is_root: bool,
    ) -> impl Future<Output = Result<Option<Admin>, RepositoryError>> + Send;

    fn delete_by_twitch_id(
        &self,
        twitch_id: &str,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
}
