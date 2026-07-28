use std::future::Future;

use crate::error::RepositoryError;
use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};

pub trait RouletteSlotRepository: Send + Sync {
    fn load_all(&self) -> impl Future<Output = Result<Vec<RouletteSlot>, RepositoryError>> + Send;
    fn save(
        &self,
        slot: RouletteSlot,
    ) -> impl Future<Output = Result<RouletteSlot, RepositoryError>> + Send;
    fn update(
        &self,
        slot: RouletteSlot,
    ) -> impl Future<Output = Result<Option<RouletteSlot>, RepositoryError>> + Send;
    fn delete(
        &self,
        id: RouletteSlotId,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
}
