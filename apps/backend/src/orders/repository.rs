use std::future::Future;
use std::sync::Arc;

use crate::error::RepositoryError;
use crate::orders::service::{OrderNormalizable, OrderReplace, OrderView};
use crate::orders::vip::{NewVipRecord, VipRecord, VipRecordId};

pub trait OrderRepository: Send + Sync {
    type Order: OrderNormalizable
        + OrderReplace<New = Self::New>
        + OrderView<Kind = Self::Kind>
        + Send;
    type New: OrderNormalizable + Send;
    type Id: Send;
    type Kind: PartialEq + Copy + Send;

    fn create(
        &self,
        order: Self::New,
    ) -> impl Future<Output = Result<Self::Order, RepositoryError>> + Send;
    fn get_by_id(
        &self,
        id: Self::Id,
    ) -> impl Future<Output = Result<Option<Self::Order>, RepositoryError>> + Send;
    fn list(&self) -> impl Future<Output = Result<Vec<Self::Order>, RepositoryError>> + Send;
    fn update(
        &self,
        order: Self::Order,
    ) -> impl Future<Output = Result<Option<Self::Order>, RepositoryError>> + Send;
    fn delete(&self, id: Self::Id) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
}

impl<T: OrderRepository + ?Sized> OrderRepository for Arc<T> {
    type Order = T::Order;
    type New = T::New;
    type Id = T::Id;
    type Kind = T::Kind;

    async fn create(&self, order: Self::New) -> Result<Self::Order, RepositoryError> {
        (**self).create(order).await
    }

    async fn get_by_id(&self, id: Self::Id) -> Result<Option<Self::Order>, RepositoryError> {
        (**self).get_by_id(id).await
    }

    async fn list(&self) -> Result<Vec<Self::Order>, RepositoryError> {
        (**self).list().await
    }

    async fn update(&self, order: Self::Order) -> Result<Option<Self::Order>, RepositoryError> {
        (**self).update(order).await
    }

    async fn delete(&self, id: Self::Id) -> Result<bool, RepositoryError> {
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
