use std::future::Future;
use std::sync::Arc;

use crate::error::RepositoryError;
use crate::orders::game::{GameOrder, GameOrderId, NewGameOrder};
use crate::orders::movie::{MovieOrder, MovieOrderId, NewMovieOrder};
use crate::orders::vip::{NewVipRecord, VipRecord, VipRecordId};

pub trait GameOrderRepository: Send + Sync {
    fn create(
        &self,
        order: NewGameOrder,
    ) -> impl Future<Output = Result<GameOrder, RepositoryError>> + Send;
    fn get_by_id(
        &self,
        id: GameOrderId,
    ) -> impl Future<Output = Result<Option<GameOrder>, RepositoryError>> + Send;
    fn list(&self) -> impl Future<Output = Result<Vec<GameOrder>, RepositoryError>> + Send;
    fn update(
        &self,
        order: GameOrder,
    ) -> impl Future<Output = Result<Option<GameOrder>, RepositoryError>> + Send;
    fn delete(&self, id: GameOrderId)
    -> impl Future<Output = Result<bool, RepositoryError>> + Send;
}

impl<T: GameOrderRepository> GameOrderRepository for Arc<T> {
    async fn create(&self, order: NewGameOrder) -> Result<GameOrder, RepositoryError> {
        (**self).create(order).await
    }

    async fn get_by_id(&self, id: GameOrderId) -> Result<Option<GameOrder>, RepositoryError> {
        (**self).get_by_id(id).await
    }

    async fn list(&self) -> Result<Vec<GameOrder>, RepositoryError> {
        (**self).list().await
    }

    async fn update(&self, order: GameOrder) -> Result<Option<GameOrder>, RepositoryError> {
        (**self).update(order).await
    }

    async fn delete(&self, id: GameOrderId) -> Result<bool, RepositoryError> {
        (**self).delete(id).await
    }
}

pub trait MovieOrderRepository: Send + Sync {
    fn create(
        &self,
        order: NewMovieOrder,
    ) -> impl Future<Output = Result<MovieOrder, RepositoryError>> + Send;
    fn get_by_id(
        &self,
        id: MovieOrderId,
    ) -> impl Future<Output = Result<Option<MovieOrder>, RepositoryError>> + Send;
    fn list(&self) -> impl Future<Output = Result<Vec<MovieOrder>, RepositoryError>> + Send;
    fn update(
        &self,
        order: MovieOrder,
    ) -> impl Future<Output = Result<Option<MovieOrder>, RepositoryError>> + Send;
    fn delete(
        &self,
        id: MovieOrderId,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
}

impl<T: MovieOrderRepository> MovieOrderRepository for Arc<T> {
    async fn create(&self, order: NewMovieOrder) -> Result<MovieOrder, RepositoryError> {
        (**self).create(order).await
    }

    async fn get_by_id(&self, id: MovieOrderId) -> Result<Option<MovieOrder>, RepositoryError> {
        (**self).get_by_id(id).await
    }

    async fn list(&self) -> Result<Vec<MovieOrder>, RepositoryError> {
        (**self).list().await
    }

    async fn update(&self, order: MovieOrder) -> Result<Option<MovieOrder>, RepositoryError> {
        (**self).update(order).await
    }

    async fn delete(&self, id: MovieOrderId) -> Result<bool, RepositoryError> {
        (**self).delete(id).await
    }
}

pub trait VipRecordRepository: Send + Sync {
    fn create(
        &self,
        record: NewVipRecord,
        end_date: chrono::NaiveDate,
    ) -> impl Future<Output = Result<VipRecord, RepositoryError>> + Send;
    fn get_by_id(
        &self,
        id: VipRecordId,
    ) -> impl Future<Output = Result<Option<VipRecord>, RepositoryError>> + Send;
    fn list(&self) -> impl Future<Output = Result<Vec<VipRecord>, RepositoryError>> + Send;
    fn update(
        &self,
        record: VipRecord,
    ) -> impl Future<Output = Result<Option<VipRecord>, RepositoryError>> + Send;
    fn delete(&self, id: VipRecordId)
    -> impl Future<Output = Result<bool, RepositoryError>> + Send;
}

impl<T: VipRecordRepository> VipRecordRepository for Arc<T> {
    async fn create(
        &self,
        record: NewVipRecord,
        end_date: chrono::NaiveDate,
    ) -> Result<VipRecord, RepositoryError> {
        (**self).create(record, end_date).await
    }

    async fn get_by_id(&self, id: VipRecordId) -> Result<Option<VipRecord>, RepositoryError> {
        (**self).get_by_id(id).await
    }

    async fn list(&self) -> Result<Vec<VipRecord>, RepositoryError> {
        (**self).list().await
    }

    async fn update(&self, record: VipRecord) -> Result<Option<VipRecord>, RepositoryError> {
        (**self).update(record).await
    }

    async fn delete(&self, id: VipRecordId) -> Result<bool, RepositoryError> {
        (**self).delete(id).await
    }
}
