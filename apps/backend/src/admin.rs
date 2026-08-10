pub mod auth;
pub mod repository;
pub mod service;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Admin {
    pub twitch_id: String,
    pub display_name: Option<String>,
    pub is_root: bool,
    pub created_at: DateTime<Utc>,
}
