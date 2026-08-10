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
mod tests {
    use std::sync::Arc;

    use crate::db::inmemory_admin::InMemoryAdminRepository;

    use super::*;

    type TestService = AdminService<InMemoryAdminRepository>;

    fn test_service() -> TestService {
        AdminService::new(Arc::new(InMemoryAdminRepository::new()))
    }

    #[tokio::test]
    async fn seed_creates_root_admin() {
        let svc = test_service();
        svc.seed("100").await.unwrap();

        let admin = svc.get("100").await.unwrap().unwrap();
        assert!(admin.is_root);
        assert!(svc.is_admin("100").await.unwrap());
        assert!(svc.is_root("100").await.unwrap());
    }

    #[tokio::test]
    async fn seed_promotes_existing_non_root() {
        let svc = test_service();
        svc.add("100", Some("sapushka_")).await.unwrap();
        assert!(!svc.is_root("100").await.unwrap());

        svc.seed("100").await.unwrap();
        assert!(svc.is_root("100").await.unwrap());
    }

    #[tokio::test]
    async fn seed_is_idempotent() {
        let svc = test_service();
        svc.seed("100").await.unwrap();
        svc.seed("100").await.unwrap();
        assert_eq!(svc.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn add_creates_non_root_admin() {
        let svc = test_service();
        let admin = svc.add("200", Some("moderator")).await.unwrap();
        assert!(!admin.is_root);
        assert!(svc.is_admin("200").await.unwrap());
        assert!(!svc.is_root("200").await.unwrap());
    }

    #[tokio::test]
    async fn add_duplicate_is_rejected() {
        let svc = test_service();
        svc.add("200", None).await.unwrap();
        let err = svc.add("200", None).await.unwrap_err();
        assert!(matches!(err, AdminServiceError::AlreadyAdmin));
    }

    #[tokio::test]
    async fn remove_deletes_admin() {
        let svc = test_service();
        svc.add("200", None).await.unwrap();
        svc.remove("200").await.unwrap();
        assert!(!svc.is_admin("200").await.unwrap());
    }

    #[tokio::test]
    async fn remove_missing_is_not_found() {
        let svc = test_service();
        let err = svc.remove("missing").await.unwrap_err();
        assert!(matches!(err, AdminServiceError::AdminNotFound));
    }

    #[tokio::test]
    async fn cannot_remove_last_root() {
        let svc = test_service();
        svc.seed("100").await.unwrap();
        let err = svc.remove("100").await.unwrap_err();
        assert!(matches!(err, AdminServiceError::CannotRemoveLastRoot));
    }

    #[tokio::test]
    async fn can_remove_first_root_when_second_exists() {
        let svc = test_service();
        svc.seed("100").await.unwrap();
        svc.add("200", None).await.unwrap();
        svc.set_root("200", true).await.unwrap();
        svc.remove("100").await.unwrap();
        assert!(svc.is_admin("200").await.unwrap());
    }

    #[tokio::test]
    async fn record_login_updates_display_name() {
        let svc = test_service();
        svc.add("200", Some("old_name")).await.unwrap();
        svc.update_display_name("200", "new_name").await.unwrap();
        let admin = svc.get("200").await.unwrap().unwrap();
        assert_eq!(admin.display_name.as_deref(), Some("new_name"));
    }
}
