use serde::de::Error;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema, strum::EnumString, strum::IntoStaticStr,
)]
#[strum(serialize_all = "snake_case")]
#[non_exhaustive]
pub enum OrderStatus {
    Pending,
    Completed,
    Cancelled,
}

impl<'de> Deserialize<'de> for OrderStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value
            .to_ascii_lowercase()
            .parse()
            .map_err(|_| Error::custom(format!("unknown order status: {value}")))
    }
}
