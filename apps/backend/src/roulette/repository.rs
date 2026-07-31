use std::future::Future;
use std::sync::Arc;

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

impl<T: RouletteSlotRepository> RouletteSlotRepository for Arc<T> {
    async fn load_all(&self) -> Result<Vec<RouletteSlot>, RepositoryError> {
        (**self).load_all().await
    }

    async fn save(&self, slot: RouletteSlot) -> Result<RouletteSlot, RepositoryError> {
        (**self).save(slot).await
    }

    async fn update(&self, slot: RouletteSlot) -> Result<Option<RouletteSlot>, RepositoryError> {
        (**self).update(slot).await
    }

    async fn delete(&self, id: RouletteSlotId) -> Result<bool, RepositoryError> {
        (**self).delete(id).await
    }
}
