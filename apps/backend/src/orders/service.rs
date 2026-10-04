use chrono::{Days, NaiveDate};

use crate::error::OrdersServiceError;
use crate::orders::game::{GameOrder, GameOrderId, GameOrderKind, NewGameOrder, OrderSource};
use crate::orders::movie::{MovieKind, MovieOrder, MovieOrderId, NewMovieOrder};
use crate::orders::repository::{GameOrderRepository, MovieOrderRepository, VipRecordRepository};
use crate::orders::status::OrderStatus;
use crate::orders::vip::{
    NewVipRecord, UNVIP_DURATION_DAYS, VIP_DURATION_DAYS, VipKind, VipRecord, VipRecordId,
    VipRecordUpdate, VipStatus,
};

fn normalize_title(title: Option<String>) -> Option<String> {
    title.and_then(|t| {
        let trimmed = t.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    normalize_title(value)
}

fn require_customer(name: &str) -> Result<String, OrdersServiceError> {
    let trimmed = name.trim().to_string();
    if trimmed.is_empty() {
        return Err(OrdersServiceError::Invalid(
            "customer name is empty".to_string(),
        ));
    }
    Ok(trimmed)
}

fn matches_query(query: &str, fields: &[&str]) -> bool {
    let needle = query.to_lowercase();
    fields.iter().any(|f| f.to_lowercase().contains(&needle))
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct GameOrderFilter {
    pub status: Option<OrderStatus>,
    pub kind: Option<GameOrderKind>,
    pub source: Option<OrderSource>,
    pub query: Option<String>,
}

#[non_exhaustive]
pub struct GameOrderService<R: GameOrderRepository> {
    repo: R,
}

impl<R: GameOrderRepository> GameOrderService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn create(&self, mut order: NewGameOrder) -> Result<GameOrder, OrdersServiceError> {
        order.customer_name = require_customer(&order.customer_name)?;
        order.title = normalize_title(order.title.take());
        order.comment = normalize_optional(order.comment.take());
        Ok(self.repo.create(order).await?)
    }

    pub async fn get_by_id(&self, id: GameOrderId) -> Result<GameOrder, OrdersServiceError> {
        self.repo
            .get_by_id(id)
            .await?
            .ok_or(OrdersServiceError::NotFound)
    }

    pub async fn list(
        &self,
        filter: GameOrderFilter,
    ) -> Result<Vec<GameOrder>, OrdersServiceError> {
        let all = self.repo.list().await?;
        Ok(all
            .into_iter()
            .filter(|o| filter.status.is_none_or(|s| o.status == s))
            .filter(|o| filter.kind.is_none_or(|k| o.kind == k))
            .filter(|o| filter.source.is_none_or(|s| o.source == s))
            .filter(|o| {
                filter.query.as_ref().is_none_or(|q| {
                    let title = o.title.as_deref().unwrap_or("");
                    matches_query(q, &[title, &o.customer_name])
                })
            })
            .rev()
            .collect())
    }

    pub async fn update(&self, mut order: GameOrder) -> Result<GameOrder, OrdersServiceError> {
        order.customer_name = require_customer(&order.customer_name)?;
        order.title = normalize_title(order.title.take());
        order.comment = normalize_optional(order.comment.take());
        self.repo
            .update(order)
            .await?
            .ok_or(OrdersServiceError::NotFound)
    }

    /// Replaces editable fields, preserving user_id and created_at.
    pub async fn replace(
        &self,
        id: GameOrderId,
        mut order: NewGameOrder,
    ) -> Result<GameOrder, OrdersServiceError> {
        let existing = self.get_by_id(id).await?;
        order.customer_name = require_customer(&order.customer_name)?;
        order.title = normalize_title(order.title.take());
        order.comment = normalize_optional(order.comment.take());
        let updated = GameOrder::new(
            id,
            order.title,
            order.customer_name,
            existing.user_id,
            order.kind,
            order.source,
            order.status,
            order.completed_at,
            order.comment,
            existing.created_at,
            existing.updated_at,
        );
        self.repo
            .update(updated)
            .await?
            .ok_or(OrdersServiceError::NotFound)
    }

    pub async fn delete(&self, id: GameOrderId) -> Result<(), OrdersServiceError> {
        if !self.repo.delete(id).await? {
            return Err(OrdersServiceError::NotFound);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct MovieOrderFilter {
    pub status: Option<OrderStatus>,
    pub kind: Option<MovieKind>,
    pub source: Option<OrderSource>,
    pub query: Option<String>,
}

#[non_exhaustive]
pub struct MovieOrderService<R: MovieOrderRepository> {
    repo: R,
}

impl<R: MovieOrderRepository> MovieOrderService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn create(&self, mut order: NewMovieOrder) -> Result<MovieOrder, OrdersServiceError> {
        order.customer_name = require_customer(&order.customer_name)?;
        order.title = normalize_title(order.title.take());
        order.comment = normalize_optional(order.comment.take());
        Ok(self.repo.create(order).await?)
    }

    pub async fn get_by_id(&self, id: MovieOrderId) -> Result<MovieOrder, OrdersServiceError> {
        self.repo
            .get_by_id(id)
            .await?
            .ok_or(OrdersServiceError::NotFound)
    }

    pub async fn list(
        &self,
        filter: MovieOrderFilter,
    ) -> Result<Vec<MovieOrder>, OrdersServiceError> {
        let all = self.repo.list().await?;
        Ok(all
            .into_iter()
            .filter(|o| filter.status.is_none_or(|s| o.status == s))
            .filter(|o| filter.kind.is_none_or(|k| o.kind == k))
            .filter(|o| filter.source.is_none_or(|s| o.source == s))
            .filter(|o| {
                filter.query.as_ref().is_none_or(|q| {
                    let title = o.title.as_deref().unwrap_or("");
                    matches_query(q, &[title, &o.customer_name])
                })
            })
            .rev()
            .collect())
    }

    pub async fn update(&self, mut order: MovieOrder) -> Result<MovieOrder, OrdersServiceError> {
        order.customer_name = require_customer(&order.customer_name)?;
        order.title = normalize_title(order.title.take());
        order.comment = normalize_optional(order.comment.take());
        self.repo
            .update(order)
            .await?
            .ok_or(OrdersServiceError::NotFound)
    }

    /// Replaces editable fields, preserving user_id and created_at.
    pub async fn replace(
        &self,
        id: MovieOrderId,
        mut order: NewMovieOrder,
    ) -> Result<MovieOrder, OrdersServiceError> {
        let existing = self.get_by_id(id).await?;
        order.customer_name = require_customer(&order.customer_name)?;
        order.title = normalize_title(order.title.take());
        order.comment = normalize_optional(order.comment.take());
        let updated = MovieOrder::new(
            id,
            order.title,
            order.customer_name,
            existing.user_id,
            order.kind,
            order.source,
            order.status,
            order.comment,
            existing.created_at,
            existing.updated_at,
        );
        self.repo
            .update(updated)
            .await?
            .ok_or(OrdersServiceError::NotFound)
    }

    pub async fn delete(&self, id: MovieOrderId) -> Result<(), OrdersServiceError> {
        if !self.repo.delete(id).await? {
            return Err(OrdersServiceError::NotFound);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct VipRecordFilter {
    pub status: Option<VipStatus>,
    pub kind: Option<VipKind>,
    pub query: Option<String>,
}

#[non_exhaustive]
pub struct VipService<R: VipRecordRepository> {
    repo: R,
}

impl<R: VipRecordRepository> VipService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn create(&self, mut record: NewVipRecord) -> Result<VipRecord, OrdersServiceError> {
        record.customer_name = require_customer(&record.customer_name)?;
        record.note = normalize_optional(record.note.take());
        let end_date = record.end_date.unwrap_or_else(|| {
            let days = match record.kind {
                VipKind::Vip => VIP_DURATION_DAYS,
                VipKind::Unvip => UNVIP_DURATION_DAYS,
            };
            record
                .roulette_date
                .checked_add_days(Days::new(days as u64))
                .unwrap_or(record.roulette_date)
        });
        if end_date < record.roulette_date {
            return Err(OrdersServiceError::Invalid(
                "end date is before roulette date".to_string(),
            ));
        }
        Ok(self.repo.create(record, end_date).await?)
    }

    pub async fn get_by_id(&self, id: VipRecordId) -> Result<VipRecord, OrdersServiceError> {
        self.repo
            .get_by_id(id)
            .await?
            .ok_or(OrdersServiceError::NotFound)
    }

    pub async fn list(
        &self,
        filter: VipRecordFilter,
    ) -> Result<Vec<VipRecord>, OrdersServiceError> {
        let all = self.repo.list().await?;
        Ok(all
            .into_iter()
            .filter(|r| filter.status.is_none_or(|s| r.status == s))
            .filter(|r| filter.kind.is_none_or(|k| r.kind == k))
            .filter(|r| {
                filter
                    .query
                    .as_ref()
                    .is_none_or(|q| matches_query(q, &[&r.customer_name]))
            })
            .rev()
            .collect())
    }

    /// Active VIP records ending within `days` from `today` (not yet ended).
    pub async fn expiring_within(
        &self,
        today: NaiveDate,
        days: u64,
    ) -> Result<Vec<VipRecord>, OrdersServiceError> {
        let horizon = today.checked_add_days(Days::new(days)).unwrap_or(today);
        let all = self.repo.list().await?;
        Ok(all
            .into_iter()
            .filter(|r| r.status == VipStatus::Active && r.kind == VipKind::Vip)
            .filter(|r| r.end_date >= today && r.end_date <= horizon)
            .collect())
    }

    /// Active Unvip records whose end date passed: VIP must be returned.
    pub async fn awaiting_return(
        &self,
        today: NaiveDate,
    ) -> Result<Vec<VipRecord>, OrdersServiceError> {
        let all = self.repo.list().await?;
        Ok(all
            .into_iter()
            .filter(|r| r.status == VipStatus::Active && r.kind == VipKind::Unvip)
            .filter(|r| r.end_date <= today)
            .collect())
    }

    pub async fn update(&self, mut record: VipRecord) -> Result<VipRecord, OrdersServiceError> {
        record.customer_name = require_customer(&record.customer_name)?;
        record.note = normalize_optional(record.note.take());
        if record.end_date < record.roulette_date {
            return Err(OrdersServiceError::Invalid(
                "end date is before roulette date".to_string(),
            ));
        }
        self.repo
            .update(record)
            .await?
            .ok_or(OrdersServiceError::NotFound)
    }

    /// Replaces editable fields, preserving user_id and created_at.
    pub async fn replace(
        &self,
        id: VipRecordId,
        mut update: VipRecordUpdate,
    ) -> Result<VipRecord, OrdersServiceError> {
        let existing = self.get_by_id(id).await?;
        update.customer_name = require_customer(&update.customer_name)?;
        update.note = normalize_optional(update.note.take());
        if update.end_date < update.roulette_date {
            return Err(OrdersServiceError::Invalid(
                "end date is before roulette date".to_string(),
            ));
        }
        let updated = VipRecord::new(
            id,
            update.customer_name,
            existing.user_id,
            update.kind,
            update.roulette_date,
            update.end_date,
            update.status,
            update.note,
            existing.created_at,
            existing.updated_at,
        );
        self.repo
            .update(updated)
            .await?
            .ok_or(OrdersServiceError::NotFound)
    }

    pub async fn delete(&self, id: VipRecordId) -> Result<(), OrdersServiceError> {
        if !self.repo.delete(id).await? {
            return Err(OrdersServiceError::NotFound);
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "service.test.rs"]
mod tests;
