use chrono::{Days, NaiveDate};

use crate::error::OrdersServiceError;
use crate::orders::game::{GameOrder, GameOrderKind, NewGameOrder, OrderSource};
use crate::orders::movie::{MovieKind, MovieOrder, NewMovieOrder};
use crate::orders::repository::{OrderRepository, VipRecordRepository};
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

pub trait OrderNormalizable {
    fn normalize(&mut self) -> Result<(), OrdersServiceError>;
}

impl OrderNormalizable for NewGameOrder {
    fn normalize(&mut self) -> Result<(), OrdersServiceError> {
        self.customer_name = require_customer(&self.customer_name)?;
        self.title = normalize_title(self.title.take());
        self.comment = normalize_optional(self.comment.take());
        Ok(())
    }
}

impl OrderNormalizable for NewMovieOrder {
    fn normalize(&mut self) -> Result<(), OrdersServiceError> {
        self.customer_name = require_customer(&self.customer_name)?;
        self.title = normalize_title(self.title.take());
        self.comment = normalize_optional(self.comment.take());
        Ok(())
    }
}

impl OrderNormalizable for GameOrder {
    fn normalize(&mut self) -> Result<(), OrdersServiceError> {
        self.customer_name = require_customer(&self.customer_name)?;
        self.title = normalize_title(self.title.take());
        self.comment = normalize_optional(self.comment.take());
        Ok(())
    }
}

impl OrderNormalizable for MovieOrder {
    fn normalize(&mut self) -> Result<(), OrdersServiceError> {
        self.customer_name = require_customer(&self.customer_name)?;
        self.title = normalize_title(self.title.take());
        self.comment = normalize_optional(self.comment.take());
        Ok(())
    }
}

pub trait OrderReplace {
    type New: OrderNormalizable;
    fn replace_from(&self, new: Self::New) -> Self;
}

impl OrderReplace for GameOrder {
    type New = NewGameOrder;
    fn replace_from(&self, new: NewGameOrder) -> Self {
        GameOrder::new(
            self.id,
            new.title,
            new.customer_name,
            self.user_id,
            new.kind,
            new.source,
            new.status,
            new.completed_at,
            new.comment,
            self.created_at,
            self.updated_at,
        )
    }
}

impl OrderReplace for MovieOrder {
    type New = NewMovieOrder;
    fn replace_from(&self, new: NewMovieOrder) -> Self {
        MovieOrder::new(
            self.id,
            new.title,
            new.customer_name,
            self.user_id,
            new.kind,
            new.source,
            new.status,
            new.comment,
            self.created_at,
            self.updated_at,
        )
    }
}

pub trait OrderView {
    type Kind: PartialEq + Copy;
    fn status(&self) -> OrderStatus;
    fn kind(&self) -> Self::Kind;
    fn source(&self) -> OrderSource;
    fn title(&self) -> Option<&str>;
    fn customer_name(&self) -> &str;
}

impl OrderView for GameOrder {
    type Kind = GameOrderKind;
    fn status(&self) -> OrderStatus {
        self.status
    }
    fn kind(&self) -> GameOrderKind {
        self.kind
    }
    fn source(&self) -> OrderSource {
        self.source
    }
    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
    fn customer_name(&self) -> &str {
        &self.customer_name
    }
}

impl OrderView for MovieOrder {
    type Kind = MovieKind;
    fn status(&self) -> OrderStatus {
        self.status
    }
    fn kind(&self) -> MovieKind {
        self.kind
    }
    fn source(&self) -> OrderSource {
        self.source
    }
    fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
    fn customer_name(&self) -> &str {
        &self.customer_name
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OrderFilter<K> {
    pub status: Option<OrderStatus>,
    pub kind: Option<K>,
    pub source: Option<OrderSource>,
    pub query: Option<String>,
}

impl<K> Default for OrderFilter<K> {
    fn default() -> Self {
        Self {
            status: None,
            kind: None,
            source: None,
            query: None,
        }
    }
}

pub type GameOrderFilter = OrderFilter<GameOrderKind>;
pub type MovieOrderFilter = OrderFilter<MovieKind>;

#[non_exhaustive]
pub struct OrderService<R: OrderRepository> {
    repo: R,
}

impl<R: OrderRepository> OrderService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn create(&self, mut order: R::New) -> Result<R::Order, OrdersServiceError> {
        order.normalize()?;
        Ok(self.repo.create(order).await?)
    }

    pub async fn get_by_id(&self, id: R::Id) -> Result<R::Order, OrdersServiceError> {
        self.repo
            .get_by_id(id)
            .await?
            .ok_or(OrdersServiceError::NotFound)
    }

    pub async fn list(
        &self,
        filter: OrderFilter<R::Kind>,
    ) -> Result<Vec<R::Order>, OrdersServiceError> {
        let all = self.repo.list().await?;
        Ok(all
            .into_iter()
            .filter(|o| filter.status.is_none_or(|s| o.status() == s))
            .filter(|o| filter.kind.is_none_or(|k| o.kind() == k))
            .filter(|o| filter.source.is_none_or(|s| o.source() == s))
            .filter(|o| {
                filter
                    .query
                    .as_ref()
                    .is_none_or(|q| matches_query(q, &[o.title().unwrap_or(""), o.customer_name()]))
            })
            .rev()
            .collect())
    }

    pub async fn update(&self, mut order: R::Order) -> Result<R::Order, OrdersServiceError> {
        order.normalize()?;
        self.repo
            .update(order)
            .await?
            .ok_or(OrdersServiceError::NotFound)
    }

    /// Replaces editable fields, preserving user_id and created_at.
    pub async fn replace(
        &self,
        id: R::Id,
        mut order: R::New,
    ) -> Result<R::Order, OrdersServiceError> {
        let existing = self.get_by_id(id).await?;
        order.normalize()?;
        let updated = existing.replace_from(order);
        self.repo
            .update(updated)
            .await?
            .ok_or(OrdersServiceError::NotFound)
    }

    pub async fn delete(&self, id: R::Id) -> Result<(), OrdersServiceError> {
        if !self.repo.delete(id).await? {
            return Err(OrdersServiceError::NotFound);
        }
        Ok(())
    }
}

pub type GameOrderService<R> = OrderService<R>;
pub type MovieOrderService<R> = OrderService<R>;

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
