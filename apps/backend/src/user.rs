pub mod repository;
pub mod service;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use utoipa::ToSchema;

use crate::platform::PlatformId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct UserId(u32);

impl UserId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }
}

impl Display for UserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct UserPlatformId(u32);

impl UserPlatformId {
    pub(crate) fn new(id: u32) -> Self {
        Self(id)
    }
}

impl Display for UserPlatformId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct User {
    pub id: UserId,
    pub display_name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct UserPlatform {
    pub id: UserPlatformId,
    pub user_id: UserId,
    pub platform_id: PlatformId,
    pub platform_user_id: String,
    pub platform_username: String,
}
