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

    pub const fn get(self) -> u32 {
        self.0
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

    pub const fn get(self) -> u32 {
        self.0
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
    _sealed: (),
}

impl User {
    pub fn new(
        id: UserId,
        display_name: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            display_name,
            created_at,
            updated_at,
            _sealed: (),
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct UserPlatform {
    pub id: UserPlatformId,
    pub user_id: UserId,
    pub platform_id: PlatformId,
    pub platform_user_id: String,
    pub platform_username: String,
    _sealed: (),
}

impl UserPlatform {
    pub fn new(
        id: UserPlatformId,
        user_id: UserId,
        platform_id: PlatformId,
        platform_user_id: String,
        platform_username: String,
    ) -> Self {
        Self {
            id,
            user_id,
            platform_id,
            platform_user_id,
            platform_username,
            _sealed: (),
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ResolvedUserPlatform {
    pub id: UserPlatformId,
    pub platform_name: String,
    pub platform_user_id: String,
    pub platform_username: String,
    _sealed: (),
}

impl ResolvedUserPlatform {
    pub fn new(
        id: UserPlatformId,
        platform_name: String,
        platform_user_id: String,
        platform_username: String,
    ) -> Self {
        Self {
            id,
            platform_name,
            platform_user_id,
            platform_username,
            _sealed: (),
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct UserView {
    pub user: User,
    pub platforms: Vec<ResolvedUserPlatform>,
    _sealed: (),
}

impl UserView {
    pub fn new(user: User, platforms: Vec<ResolvedUserPlatform>) -> Self {
        Self {
            user,
            platforms,
            _sealed: (),
        }
    }
}
