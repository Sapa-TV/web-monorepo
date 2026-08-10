use std::sync::nonpoison::Mutex;

use chrono::Utc;

use crate::admin::Admin;
use crate::admin::repository::AdminRepository;
use crate::error::RepositoryError;

#[non_exhaustive]
pub struct InMemoryAdminRepository {
    admins: Mutex<Vec<Admin>>,
}

impl InMemoryAdminRepository {
    pub fn new() -> Self {
        Self {
            admins: Mutex::new(Vec::new()),
        }
    }
}

impl Default for InMemoryAdminRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl AdminRepository for InMemoryAdminRepository {
    async fn create(
        &self,
        twitch_id: &str,
        display_name: Option<&str>,
        is_root: bool,
    ) -> Result<Admin, RepositoryError> {
        let mut admins = self.admins.lock();
        if admins.iter().any(|a| a.twitch_id == twitch_id) {
            return Err(RepositoryError::Conflict(
                "admin with this twitch_id already exists".to_string(),
            ));
        }
        let admin = Admin {
            twitch_id: twitch_id.to_string(),
            display_name: display_name.map(str::to_string),
            is_root,
            created_at: Utc::now(),
        };
        admins.push(admin.clone());
        Ok(admin)
    }

    async fn get_by_twitch_id(&self, twitch_id: &str) -> Result<Option<Admin>, RepositoryError> {
        Ok(self
            .admins
            .lock()
            .iter()
            .find(|a| a.twitch_id == twitch_id)
            .cloned())
    }

    async fn list(&self) -> Result<Vec<Admin>, RepositoryError> {
        Ok(self.admins.lock().clone())
    }

    async fn update_display_name(
        &self,
        twitch_id: &str,
        display_name: &str,
    ) -> Result<Option<Admin>, RepositoryError> {
        let mut admins = self.admins.lock();
        let Some(admin) = admins.iter_mut().find(|a| a.twitch_id == twitch_id) else {
            return Ok(None);
        };
        admin.display_name = Some(display_name.to_string());
        Ok(Some(admin.clone()))
    }

    async fn set_root(
        &self,
        twitch_id: &str,
        is_root: bool,
    ) -> Result<Option<Admin>, RepositoryError> {
        let mut admins = self.admins.lock();
        let Some(admin) = admins.iter_mut().find(|a| a.twitch_id == twitch_id) else {
            return Ok(None);
        };
        admin.is_root = is_root;
        Ok(Some(admin.clone()))
    }

    async fn delete_by_twitch_id(&self, twitch_id: &str) -> Result<bool, RepositoryError> {
        let mut admins = self.admins.lock();
        let len_before = admins.len();
        admins.retain(|a| a.twitch_id != twitch_id);
        Ok(admins.len() != len_before)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_and_get() {
        let repo = InMemoryAdminRepository::new();
        let admin = repo.create("100", Some("sapushka_"), true).await.unwrap();
        assert!(admin.is_root);

        let fetched = repo.get_by_twitch_id("100").await.unwrap().unwrap();
        assert_eq!(fetched.twitch_id, "100");
        assert_eq!(fetched.display_name.as_deref(), Some("sapushka_"));
    }

    #[tokio::test]
    async fn create_conflicts_on_duplicate_id() {
        let repo = InMemoryAdminRepository::new();
        repo.create("100", None, true).await.unwrap();
        let err = repo.create("100", None, false).await.unwrap_err();
        assert!(matches!(err, RepositoryError::Conflict(_)));
    }

    #[tokio::test]
    async fn list_returns_all() {
        let repo = InMemoryAdminRepository::new();
        repo.create("100", None, true).await.unwrap();
        repo.create("200", None, false).await.unwrap();
        assert_eq!(repo.list().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn set_root_flips_flag() {
        let repo = InMemoryAdminRepository::new();
        repo.create("200", None, false).await.unwrap();
        let admin = repo.set_root("200", true).await.unwrap().unwrap();
        assert!(admin.is_root);
        assert!(!repo.set_root("missing", true).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn update_display_name() {
        let repo = InMemoryAdminRepository::new();
        repo.create("200", Some("old"), false).await.unwrap();
        let admin = repo
            .update_display_name("200", "new")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(admin.display_name.as_deref(), Some("new"));
        assert!(
            repo.update_display_name("missing", "x")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn delete_removes_entry() {
        let repo = InMemoryAdminRepository::new();
        repo.create("200", None, false).await.unwrap();
        assert!(repo.delete_by_twitch_id("200").await.unwrap());
        assert!(!repo.delete_by_twitch_id("200").await.unwrap());
        assert!(repo.get_by_twitch_id("200").await.unwrap().is_none());
    }
}
