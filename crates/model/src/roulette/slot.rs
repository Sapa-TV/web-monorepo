use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use utoipa::ToSchema;

use crate::roulette::rarity::RarityId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
#[non_exhaustive]
pub struct RouletteSlotId(u32);

impl RouletteSlotId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl Display for RouletteSlotId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, serde::Serialize, ToSchema)]
#[non_exhaustive]
pub struct RouletteSlot {
    pub id: RouletteSlotId,
    pub name: String,
    pub rarity_id: RarityId,
    pub weight: u64,
    pub action: String,
    #[serde(skip)]
    _sealed: (),
}

impl RouletteSlot {
    pub fn new<S1, S2>(
        id: RouletteSlotId,
        name: S1,
        rarity_id: RarityId,
        weight: u64,
        action: S2,
    ) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self {
            id,
            name: name.into(),
            rarity_id,
            weight,
            action: action.into(),
            _sealed: (),
        }
    }
}
