pub mod auth;
pub mod csrf;
pub mod repository;
pub mod service;
pub mod vk_auth;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Admin {
    pub twitch_id: String,
    pub display_name: Option<String>,
    pub is_root: bool,
    pub created_at: DateTime<Utc>,
    _sealed: (),
}

impl Admin {
    pub fn new(
        twitch_id: String,
        display_name: Option<String>,
        is_root: bool,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            twitch_id,
            display_name,
            is_root,
            created_at,
            _sealed: (),
        }
    }
}
