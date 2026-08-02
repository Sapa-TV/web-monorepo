use std::fmt::{self, Display};
use std::future::Future;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::RepositoryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct RarityId(u32);

impl RarityId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }
}

impl Display for RarityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Rarity {
    pub(crate) id: RarityId,
    pub(crate) name: String,
    pub(crate) display_name: String,
    pub(crate) image: String,
    pub(crate) color: String,
}

impl Rarity {
    pub fn new<S: Into<String>>(
        id: RarityId,
        name: S,
        display_name: S,
        image: S,
        color: S,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            display_name: display_name.into(),
            image: image.into(),
            color: color.into(),
        }
    }
}

pub trait RarityRepository: Send + Sync {
    fn load_all(&self) -> impl Future<Output = Result<Vec<Rarity>, RepositoryError>> + Send;
    fn save(&self, rarity: Rarity) -> impl Future<Output = Result<Rarity, RepositoryError>> + Send;
    fn update(
        &self,
        rarity: Rarity,
    ) -> impl Future<Output = Result<Option<Rarity>, RepositoryError>> + Send;
    fn delete(&self, id: RarityId) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
}

impl<T: RarityRepository> RarityRepository for Arc<T> {
    async fn load_all(&self) -> Result<Vec<Rarity>, RepositoryError> {
        (**self).load_all().await
    }

    async fn save(&self, rarity: Rarity) -> Result<Rarity, RepositoryError> {
        (**self).save(rarity).await
    }

    async fn update(&self, rarity: Rarity) -> Result<Option<Rarity>, RepositoryError> {
        (**self).update(rarity).await
    }

    async fn delete(&self, id: RarityId) -> Result<bool, RepositoryError> {
        (**self).delete(id).await
    }
}
