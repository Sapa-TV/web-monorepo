use std::sync::Arc;

use crate::admin::Admin;
use crate::admin::repository::AdminRepository;
use crate::error::AdminServiceError;
use crate::error::RepositoryError;

#[non_exhaustive]
pub struct AdminService<A>
where
    A: AdminRepository,
{
    repo: Arc<A>,
}

impl<A> AdminService<A>
where
    A: AdminRepository,
{
    pub fn new(repo: Arc<A>) -> Self {
        Self { repo }
    }

    pub async fn seed(&self, twitch_id: &str) -> Result<(), RepositoryError> {
        let existing = self.repo.get_by_twitch_id(twitch_id).await?;
        match existing {
            Some(admin) if admin.is_root => {}
            Some(_) => {
                self.repo.set_root(twitch_id, true).await?;
            }
            None => {
                self.repo.create(twitch_id, None, true).await?;
            }
        }
        Ok(())
    }

    /// Idempotent non-root seed: creates a regular admin, never demotes an
    /// existing (root) admin. Used for env-provided extra admins.
    pub async fn seed_non_root(&self, twitch_id: &str) -> Result<(), RepositoryError> {
        if self.repo.get_by_twitch_id(twitch_id).await?.is_none() {
            self.repo.create(twitch_id, None, false).await?;
        }
        Ok(())
    }

    pub async fn is_admin(&self, twitch_id: &str) -> Result<bool, AdminServiceError> {
        Ok(self.repo.get_by_twitch_id(twitch_id).await?.is_some())
    }

    pub async fn is_root(&self, twitch_id: &str) -> Result<bool, AdminServiceError> {
        Ok(self
            .repo
            .get_by_twitch_id(twitch_id)
            .await?
            .map(|admin| admin.is_root)
            .unwrap_or(false))
    }

    pub async fn get(&self, twitch_id: &str) -> Result<Option<Admin>, AdminServiceError> {
        Ok(self.repo.get_by_twitch_id(twitch_id).await?)
    }

    pub async fn list(&self) -> Result<Vec<Admin>, AdminServiceError> {
        Ok(self.repo.list().await?)
    }

    pub async fn add(
        &self,
        twitch_id: &str,
        display_name: Option<&str>,
    ) -> Result<Admin, AdminServiceError> {
        if self.repo.get_by_twitch_id(twitch_id).await?.is_some() {
            return Err(AdminServiceError::AlreadyAdmin);
        }
        Ok(self.repo.create(twitch_id, display_name, false).await?)
    }

    pub async fn set_root(
        &self,
        twitch_id: &str,
        is_root: bool,
    ) -> Result<Option<Admin>, AdminServiceError> {
        Ok(self.repo.set_root(twitch_id, is_root).await?)
    }

    pub async fn update_display_name(
        &self,
        twitch_id: &str,
        display_name: &str,
    ) -> Result<(), AdminServiceError> {
        if self.repo.get_by_twitch_id(twitch_id).await?.is_none() {
            return Err(AdminServiceError::AdminNotFound);
        }
        self.repo
            .update_display_name(twitch_id, display_name)
            .await?;
        Ok(())
    }

    pub async fn remove(&self, twitch_id: &str) -> Result<(), AdminServiceError> {
        let Some(admin) = self.repo.get_by_twitch_id(twitch_id).await? else {
            return Err(AdminServiceError::AdminNotFound);
        };
        if admin.is_root {
            let roots = self
                .repo
                .list()
                .await?
                .into_iter()
                .filter(|a| a.is_root)
                .count();
            if roots <= 1 {
                return Err(AdminServiceError::CannotRemoveLastRoot);
            }
        }
        self.repo.delete_by_twitch_id(twitch_id).await?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "service.test.rs"]
mod tests;
