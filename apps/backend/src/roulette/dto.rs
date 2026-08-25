use serde::Serialize;
use utoipa::ToSchema;

use crate::roulette::rarity::{Rarity, RarityId};
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct RarityResponse {
    pub id: RarityId,
    pub name: String,
    pub display_name: String,
    pub image: String,
    pub color: String,
    #[serde(skip)]
    _sealed: (),
}

impl From<Rarity> for RarityResponse {
    fn from(r: Rarity) -> Self {
        Self {
            id: r.id,
            name: r.name,
            display_name: r.display_name,
            image: r.image,
            color: r.color,
            _sealed: (),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct RouletteSlotResponse {
    pub id: RouletteSlotId,
    pub name: String,
    pub rarity_id: RarityId,
    pub weight: u64,
    pub action: String,
    #[serde(skip)]
    _sealed: (),
}

impl From<RouletteSlot> for RouletteSlotResponse {
    fn from(slot: RouletteSlot) -> Self {
        Self {
            id: slot.id,
            name: slot.name,
            rarity_id: slot.rarity_id,
            weight: slot.weight,
            action: slot.action,
            _sealed: (),
        }
    }
}
