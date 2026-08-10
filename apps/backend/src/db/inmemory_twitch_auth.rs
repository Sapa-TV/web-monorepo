use std::sync::nonpoison::Mutex;

use crate::error::RepositoryError;
use crate::ingress::twitch_auth::TwitchTokenRepository;

#[non_exhaustive]
pub struct InMemoryTwitchTokenRepository {
    token: Mutex<Option<String>>,
}

impl InMemoryTwitchTokenRepository {
    pub fn new() -> Self {
        Self {
            token: Mutex::new(None),
        }
    }

    pub fn seeded(token: impl Into<String>) -> Self {
        Self {
            token: Mutex::new(Some(token.into())),
        }
    }
}

impl Default for InMemoryTwitchTokenRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl TwitchTokenRepository for InMemoryTwitchTokenRepository {
    async fn load(&self) -> Result<Option<String>, RepositoryError> {
        Ok(self.token.lock().clone())
    }

    async fn save(&self, refresh_token: &str) -> Result<(), RepositoryError> {
        *self.token.lock() = Some(refresh_token.trim().to_string());
        Ok(())
    }

    async fn clear(&self) -> Result<(), RepositoryError> {
        *self.token.lock() = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn roundtrip() {
        let repo = InMemoryTwitchTokenRepository::new();
        assert_eq!(repo.load().await.unwrap(), None);

        repo.save("refresh_token_1").await.unwrap();
        assert_eq!(
            repo.load().await.unwrap().as_deref(),
            Some("refresh_token_1")
        );
    }

    #[tokio::test]
    async fn clear_removes_token() {
        let repo = InMemoryTwitchTokenRepository::new();
        repo.save("token").await.unwrap();
        repo.clear().await.unwrap();
        assert_eq!(repo.load().await.unwrap(), None);
    }
}
