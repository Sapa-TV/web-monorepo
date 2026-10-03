use chrono::{DateTime, Utc};
use serde::de::Error;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use utoipa::ToSchema;

use crate::orders::game::OrderSource;
use crate::orders::status::OrderStatus;
use crate::user::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct MovieOrderId(u32);

impl MovieOrderId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Display for MovieOrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema, strum::EnumString, strum::IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MovieKind {
    Movie,
    Series,
    Anime,
    Youtube,
}

impl<'de> Deserialize<'de> for MovieKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value
            .to_ascii_lowercase()
            .parse()
            .map_err(|_| Error::custom(format!("unknown movie kind: {value}")))
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MovieOrder {
    pub id: MovieOrderId,
    pub title: Option<String>,
    pub customer_name: String,
    pub user_id: Option<UserId>,
    pub kind: MovieKind,
    pub source: OrderSource,
    pub status: OrderStatus,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    _sealed: (),
}

impl MovieOrder {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: MovieOrderId,
        title: Option<String>,
        customer_name: String,
        user_id: Option<UserId>,
        kind: MovieKind,
        source: OrderSource,
        status: OrderStatus,
        comment: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            title,
            customer_name,
            user_id,
            kind,
            source,
            status,
            comment,
            created_at,
            updated_at,
            _sealed: (),
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NewMovieOrder {
    pub title: Option<String>,
    pub customer_name: String,
    pub user_id: Option<UserId>,
    pub kind: MovieKind,
    pub source: OrderSource,
    pub status: OrderStatus,
    pub comment: Option<String>,
    _sealed: (),
}

impl NewMovieOrder {
    pub fn new(
        title: Option<String>,
        customer_name: String,
        user_id: Option<UserId>,
        kind: MovieKind,
        source: OrderSource,
        status: OrderStatus,
        comment: Option<String>,
    ) -> Self {
        Self {
            title,
            customer_name,
            user_id,
            kind,
            source,
            status,
            comment,
            _sealed: (),
        }
    }
}
