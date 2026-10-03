use chrono::{DateTime, NaiveDate, Utc};
use serde::de::Error;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use utoipa::ToSchema;

use crate::orders::status::OrderStatus;
use crate::user::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct GameOrderId(u32);

impl GameOrderId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Display for GameOrderId {
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
pub enum GameOrderKind {
    Stream,
    Playthrough,
}

impl<'de> Deserialize<'de> for GameOrderKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value
            .to_ascii_lowercase()
            .parse()
            .map_err(|_| Error::custom(format!("unknown game order kind: {value}")))
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema, strum::EnumString, strum::IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OrderSource {
    Donate,
    Points,
    Roulette,
    Other,
}

impl<'de> Deserialize<'de> for OrderSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value
            .to_ascii_lowercase()
            .parse()
            .map_err(|_| Error::custom(format!("unknown order source: {value}")))
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GameOrder {
    pub id: GameOrderId,
    pub title: Option<String>,
    pub customer_name: String,
    pub user_id: Option<UserId>,
    pub kind: GameOrderKind,
    pub source: OrderSource,
    pub status: OrderStatus,
    pub completed_at: Option<NaiveDate>,
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    _sealed: (),
}

impl GameOrder {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: GameOrderId,
        title: Option<String>,
        customer_name: String,
        user_id: Option<UserId>,
        kind: GameOrderKind,
        source: OrderSource,
        status: OrderStatus,
        completed_at: Option<NaiveDate>,
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
            completed_at,
            comment,
            created_at,
            updated_at,
            _sealed: (),
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NewGameOrder {
    pub title: Option<String>,
    pub customer_name: String,
    pub user_id: Option<UserId>,
    pub kind: GameOrderKind,
    pub source: OrderSource,
    pub status: OrderStatus,
    pub completed_at: Option<NaiveDate>,
    pub comment: Option<String>,
    _sealed: (),
}

impl NewGameOrder {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        title: Option<String>,
        customer_name: String,
        user_id: Option<UserId>,
        kind: GameOrderKind,
        source: OrderSource,
        status: OrderStatus,
        completed_at: Option<NaiveDate>,
        comment: Option<String>,
    ) -> Self {
        Self {
            title,
            customer_name,
            user_id,
            kind,
            source,
            status,
            completed_at,
            comment,
            _sealed: (),
        }
    }
}
