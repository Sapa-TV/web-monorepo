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

    pub const TWITCH: PlatformId = PlatformId::new(1);
    pub const YOUTUBE: PlatformId = PlatformId::new(2);
    pub const VK_VIDEO_LIVE: PlatformId = PlatformId::new(3);

    pub const fn name(self) -> &'static str {
        match self.0 {
            1 => "twitch",
            2 => "youtube",
            3 => "vk_video_live",
            _ => "unknown",
        }
    }
}

impl Display for PlatformId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    pub fn from_id(id: PlatformId) -> Self {
        Self::new(id, id.name())
    }

    pub fn as_name(&self) -> &'static str {
        self.id.name()
    }
}

impl Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_name())
    }
}

pub trait PlatformCredentialRepository: Send + Sync {
    fn load_credential(
        &self,
        platform: PlatformId,
    ) -> impl Future<Output = Result<Option<String>, RepositoryError>> + Send;
    fn save_credential(
        &self,
        platform: PlatformId,
        credential: &str,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
    fn clear_credential(
        &self,
        platform: PlatformId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;
}

pub trait PlatformRepository: Send + Sync {
    fn find_by_name(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Option<Platform>, RepositoryError>> + Send;
    fn load_all(&self) -> impl Future<Output = Result<Vec<Platform>, RepositoryError>> + Send;
}
