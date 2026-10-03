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
            .collect())
    }

    /// Active VIP records expiring within `days` from `today` (inclusive).
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
            .filter(|r| r.end_date <= horizon)
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
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::db::inmemory_orders::{
        InMemoryGameOrderRepository, InMemoryMovieOrderRepository, InMemoryVipRecordRepository,
    };

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn new_game(title: Option<&str>, customer: &str) -> NewGameOrder {
        NewGameOrder::new(
            title.map(str::to_string),
            customer.to_string(),
            None,
            GameOrderKind::Stream,
            OrderSource::Roulette,
            OrderStatus::Pending,
            None,
            None,
        )
    }

    #[tokio::test]
    async fn game_create_trims_and_validates() {
        let svc = GameOrderService::new(InMemoryGameOrderRepository::new());

        let created = svc
            .create(new_game(Some("  "), "  ninja_number_one  "))
            .await
            .unwrap();
        assert_eq!(created.title, None);
        assert_eq!(created.customer_name, "ninja_number_one");

        let err = svc.create(new_game(Some("x"), "   ")).await.unwrap_err();
        assert!(matches!(err, OrdersServiceError::Invalid(_)));
    }

    #[tokio::test]
    async fn game_list_filters_and_searches() {
        let svc = GameOrderService::new(InMemoryGameOrderRepository::new());
        svc.create(new_game(Some("Noita"), "Rikrims"))
            .await
            .unwrap();
        let mut second = new_game(Some("Starcraft 2"), "filneyner");
        second.kind = GameOrderKind::Playthrough;
        second.source = OrderSource::Donate;
        svc.create(second).await.unwrap();
        let mut third = new_game(None, "Jeker3");
        third.status = OrderStatus::Cancelled;
        svc.create(third).await.unwrap();

        let pending = svc
            .list(GameOrderFilter {
                status: Some(OrderStatus::Pending),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(pending.len(), 2);

        let playthroughs = svc
            .list(GameOrderFilter {
                kind: Some(GameOrderKind::Playthrough),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(playthroughs.len(), 1);
        assert_eq!(playthroughs[0].customer_name, "filneyner");

        let found = svc
            .list(GameOrderFilter {
                query: Some("star".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(found.len(), 1);

        let by_customer = svc
            .list(GameOrderFilter {
                query: Some("JEKER".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(by_customer.len(), 1);
        assert_eq!(by_customer[0].title, None);
    }

    #[tokio::test]
    async fn movie_crud_roundtrip() {
        let svc = MovieOrderService::new(InMemoryMovieOrderRepository::new());
        let created = svc
            .create(NewMovieOrder::new(
                Some("Большой куш".to_string()),
                "ViyScar".to_string(),
                None,
                MovieKind::Movie,
                OrderSource::Donate,
                OrderStatus::Pending,
                None,
            ))
            .await
            .unwrap();

        let mut updated = created.clone();
        updated.status = OrderStatus::Completed;
        let updated = svc.update(updated).await.unwrap();
        assert_eq!(updated.status, OrderStatus::Completed);

        svc.delete(created.id).await.unwrap();
        let err = svc.get_by_id(created.id).await.unwrap_err();
        assert!(matches!(err, OrdersServiceError::NotFound));
    }

    #[tokio::test]
    async fn vip_default_end_dates_by_kind() {
        let svc = VipService::new(InMemoryVipRecordRepository::new());
        let start = date("2026-10-01");

        let vip = svc
            .create(NewVipRecord::new(
                "kasperaas".to_string(),
                None,
                VipKind::Vip,
                start,
                None,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(vip.end_date, date("2026-10-15"));
        assert_eq!(vip.status, VipStatus::Active);

        let unvip = svc
            .create(NewVipRecord::new(
                "JackTheRizer".to_string(),
                None,
                VipKind::Unvip,
                start,
                None,
                None,
            ))
            .await
            .unwrap();
        assert_eq!(unvip.end_date, date("2026-10-08"));

        let err = svc
            .create(NewVipRecord::new(
                "x".to_string(),
                None,
                VipKind::Vip,
                start,
                Some(date("2026-09-30")),
                None,
            ))
            .await
            .unwrap_err();
        assert!(matches!(err, OrdersServiceError::Invalid(_)));
    }

    #[tokio::test]
    async fn vip_reminders() {
        let svc = VipService::new(InMemoryVipRecordRepository::new());
        let today = date("2026-10-10");

        svc.create(NewVipRecord::new(
            "soon".to_string(),
            None,
            VipKind::Vip,
            date("2026-09-30"),
            Some(date("2026-10-12")),
            None,
        ))
        .await
        .unwrap();
        svc.create(NewVipRecord::new(
            "far".to_string(),
            None,
            VipKind::Vip,
            date("2026-10-09"),
            Some(date("2026-10-30")),
            None,
        ))
        .await
        .unwrap();
        svc.create(NewVipRecord::new(
            "loser".to_string(),
            None,
            VipKind::Unvip,
            date("2026-10-01"),
            Some(date("2026-10-08")),
            None,
        ))
        .await
        .unwrap();

        let expiring = svc.expiring_within(today, 3).await.unwrap();
        assert_eq!(expiring.len(), 1);
        assert_eq!(expiring[0].customer_name, "soon");

        let awaiting = svc.awaiting_return(today).await.unwrap();
        assert_eq!(awaiting.len(), 1);
        assert_eq!(awaiting[0].customer_name, "loser");
    }
}
