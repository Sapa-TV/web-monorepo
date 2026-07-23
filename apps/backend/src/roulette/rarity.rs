use std::future::Future;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::RepositoryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct RarityId(u32);

impl RarityId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }

    pub(crate) const fn value(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone)]
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
    ) -> impl Future<Output = Result<Rarity, RepositoryError>> + Send;
    fn delete(&self, id: RarityId) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}
