pub mod repository;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::platform::PlatformId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
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
pub struct User {
    pub id: UserId,
    pub display_name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub struct UserPlatform {
    pub id: UserPlatformId,
    pub user_id: UserId,
    pub platform_id: PlatformId,
    pub platform_user_id: String,
    pub platform_username: String,
}
