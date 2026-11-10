use std::future::Future;

use crate::error::RepositoryError;
use crate::push::{NewPushSubscription, PushSubscription, PushSubscriptionId};
use crate::user::UserId;

pub trait PushSubscriptionRepository: Send + Sync {
    fn upsert(
        &self,
        subscription: &NewPushSubscription,
    ) -> impl Future<Output = Result<PushSubscription, RepositoryError>> + Send;

    fn list_all(
        &self,
    ) -> impl Future<Output = Result<Vec<PushSubscription>, RepositoryError>> + Send;

    fn list_by_user(
        &self,
        user_id: UserId,
    ) -> impl Future<Output = Result<Vec<PushSubscription>, RepositoryError>> + Send;

    fn delete_by_user_and_endpoint(
        &self,
        user_id: UserId,
        endpoint: &str,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;

    fn record_delivery(
        &self,
        id: PushSubscriptionId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;

    fn record_failure(
        &self,
        id: PushSubscriptionId,
    ) -> impl Future<Output = Result<(), RepositoryError>> + Send;

    fn delete(
        &self,
        id: PushSubscriptionId,
    ) -> impl Future<Output = Result<bool, RepositoryError>> + Send;
}
