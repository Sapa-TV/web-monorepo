use chrono::{DateTime, NaiveDate, Utc};
use serde::de::Error;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use utoipa::ToSchema;

use crate::user::UserId;

pub const VIP_DURATION_DAYS: i64 = 14;
pub const UNVIP_DURATION_DAYS: i64 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct VipRecordId(u32);

impl VipRecordId {
    pub(crate) const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Display for VipRecordId {
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
pub enum VipKind {
    Vip,
    Unvip,
}

impl<'de> Deserialize<'de> for VipKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value
            .to_ascii_lowercase()
            .parse()
            .map_err(|_| Error::custom(format!("unknown vip kind: {value}")))
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema, strum::EnumString, strum::IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum VipStatus {
    Active,
    Done,
    Cancelled,
}

impl<'de> Deserialize<'de> for VipStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value
            .to_ascii_lowercase()
            .parse()
            .map_err(|_| Error::custom(format!("unknown vip status: {value}")))
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct VipRecord {
    pub id: VipRecordId,
    pub customer_name: String,
    pub user_id: Option<UserId>,
    pub kind: VipKind,
    pub roulette_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: VipStatus,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    _sealed: (),
}

impl VipRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: VipRecordId,
        customer_name: String,
        user_id: Option<UserId>,
        kind: VipKind,
        roulette_date: NaiveDate,
        end_date: NaiveDate,
        status: VipStatus,
        note: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            customer_name,
            user_id,
            kind,
            roulette_date,
            end_date,
            status,
            note,
            created_at,
            updated_at,
            _sealed: (),
        }
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NewVipRecord {
    pub customer_name: String,
    pub user_id: Option<UserId>,
    pub kind: VipKind,
    pub roulette_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub note: Option<String>,
    _sealed: (),
}

impl NewVipRecord {
    pub fn new(
        customer_name: String,
        user_id: Option<UserId>,
        kind: VipKind,
        roulette_date: NaiveDate,
        end_date: Option<NaiveDate>,
        note: Option<String>,
    ) -> Self {
        Self {
            customer_name,
            user_id,
            kind,
            roulette_date,
            end_date,
            note,
            _sealed: (),
        }
    }
}
