pub mod repository;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
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

    pub(crate) const fn value(&self) -> u32 {
        self.0
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

    pub(crate) fn value(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct User {
    pub id: UserId,
    pub display_name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl User {
    pub fn new(
        id: UserId,
        display_name: impl Into<String>,
        created_at: NaiveDateTime,
        updated_at: NaiveDateTime,
    ) -> Self {
        Self {
            id,
            display_name: display_name.into(),
            created_at,
            updated_at,
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
}

impl UserPlatform {
    pub fn new(
        id: UserPlatformId,
        user_id: UserId,
        platform_id: PlatformId,
        platform_user_id: impl Into<String>,
        platform_username: impl Into<String>,
    ) -> Self {
        Self {
            id,
            user_id,
            platform_id,
            platform_user_id: platform_user_id.into(),
            platform_username: platform_username.into(),
        }
    }
}
