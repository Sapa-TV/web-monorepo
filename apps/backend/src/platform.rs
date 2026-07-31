use std::fmt::{self, Display};
use std::future::Future;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::RepositoryError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct PlatformId(u32);

impl PlatformId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }
}

impl Display for PlatformId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Platform {
    pub id: PlatformId,
    pub name: String,
}

impl Platform {
    pub fn new(id: PlatformId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }
}

pub trait PlatformRepository: Send + Sync {
    fn find_by_name(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Option<Platform>, RepositoryError>> + Send;
    fn load_all(&self) -> impl Future<Output = Result<Vec<Platform>, RepositoryError>> + Send;
}
