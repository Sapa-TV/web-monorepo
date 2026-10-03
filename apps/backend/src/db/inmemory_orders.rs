use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::nonpoison::Mutex;

use chrono::{NaiveDate, Utc};

use crate::error::RepositoryError;
use crate::orders::game::{GameOrder, GameOrderId, NewGameOrder};
use crate::orders::movie::{MovieOrder, MovieOrderId, NewMovieOrder};
use crate::orders::repository::{GameOrderRepository, MovieOrderRepository, VipRecordRepository};
use crate::orders::vip::{NewVipRecord, VipRecord, VipRecordId, VipStatus};

#[non_exhaustive]
pub struct InMemoryGameOrderRepository {
    orders: Mutex<Vec<GameOrder>>,
    next_id: AtomicU32,
}

impl Default for InMemoryGameOrderRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryGameOrderRepository {
    pub fn new() -> Self {
        Self {
            orders: Mutex::new(Vec::new()),
            next_id: AtomicU32::new(1),
        }
    }
}

impl GameOrderRepository for InMemoryGameOrderRepository {
    async fn create(&self, order: NewGameOrder) -> Result<GameOrder, RepositoryError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Utc::now();
        let created = GameOrder::new(
            GameOrderId::new(id),
            order.title,
            order.customer_name,
            order.user_id,
            order.kind,
            order.source,
            order.status,
            order.completed_at,
            order.comment,
            now,
            now,
        );
        self.orders.lock().push(created.clone());
        Ok(created)
    }

    async fn get_by_id(&self, id: GameOrderId) -> Result<Option<GameOrder>, RepositoryError> {
        Ok(self.orders.lock().iter().find(|o| o.id == id).cloned())
    }

    async fn list(&self) -> Result<Vec<GameOrder>, RepositoryError> {
        Ok(self.orders.lock().clone())
    }

    async fn update(&self, order: GameOrder) -> Result<Option<GameOrder>, RepositoryError> {
        let mut orders = self.orders.lock();
        let Some(existing) = orders.iter_mut().find(|o| o.id == order.id) else {
            return Ok(None);
        };
        let updated = GameOrder::new(
            order.id,
            order.title,
            order.customer_name,
            order.user_id,
            order.kind,
            order.source,
            order.status,
            order.completed_at,
            order.comment,
            existing.created_at,
            Utc::now(),
        );
        *existing = updated.clone();
        Ok(Some(updated))
    }

    async fn delete(&self, id: GameOrderId) -> Result<bool, RepositoryError> {
        let mut orders = self.orders.lock();
        let len_before = orders.len();
        orders.retain(|o| o.id != id);
        Ok(orders.len() != len_before)
    }
}

#[non_exhaustive]
pub struct InMemoryMovieOrderRepository {
    orders: Mutex<Vec<MovieOrder>>,
    next_id: AtomicU32,
}

impl Default for InMemoryMovieOrderRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryMovieOrderRepository {
    pub fn new() -> Self {
        Self {
            orders: Mutex::new(Vec::new()),
            next_id: AtomicU32::new(1),
        }
    }
}

impl MovieOrderRepository for InMemoryMovieOrderRepository {
    async fn create(&self, order: NewMovieOrder) -> Result<MovieOrder, RepositoryError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Utc::now();
        let created = MovieOrder::new(
            MovieOrderId::new(id),
            order.title,
            order.customer_name,
            order.user_id,
            order.kind,
            order.source,
            order.status,
            order.comment,
            now,
            now,
        );
        self.orders.lock().push(created.clone());
        Ok(created)
    }

    async fn get_by_id(&self, id: MovieOrderId) -> Result<Option<MovieOrder>, RepositoryError> {
        Ok(self.orders.lock().iter().find(|o| o.id == id).cloned())
    }

    async fn list(&self) -> Result<Vec<MovieOrder>, RepositoryError> {
        Ok(self.orders.lock().clone())
    }

    async fn update(&self, order: MovieOrder) -> Result<Option<MovieOrder>, RepositoryError> {
        let mut orders = self.orders.lock();
        let Some(existing) = orders.iter_mut().find(|o| o.id == order.id) else {
            return Ok(None);
        };
        let updated = MovieOrder::new(
            order.id,
            order.title,
            order.customer_name,
            order.user_id,
            order.kind,
            order.source,
            order.status,
            order.comment,
            existing.created_at,
            Utc::now(),
        );
        *existing = updated.clone();
        Ok(Some(updated))
    }

    async fn delete(&self, id: MovieOrderId) -> Result<bool, RepositoryError> {
        let mut orders = self.orders.lock();
        let len_before = orders.len();
        orders.retain(|o| o.id != id);
        Ok(orders.len() != len_before)
    }
}

#[non_exhaustive]
pub struct InMemoryVipRecordRepository {
    records: Mutex<Vec<VipRecord>>,
    next_id: AtomicU32,
}

impl Default for InMemoryVipRecordRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryVipRecordRepository {
    pub fn new() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
            next_id: AtomicU32::new(1),
        }
    }
}

impl VipRecordRepository for InMemoryVipRecordRepository {
    async fn create(
        &self,
        record: NewVipRecord,
        end_date: NaiveDate,
    ) -> Result<VipRecord, RepositoryError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Utc::now();
        let created = VipRecord::new(
            VipRecordId::new(id),
            record.customer_name,
            record.user_id,
            record.kind,
            record.roulette_date,
            end_date,
            VipStatus::Active,
            record.note,
            now,
            now,
        );
        self.records.lock().push(created.clone());
        Ok(created)
    }

    async fn get_by_id(&self, id: VipRecordId) -> Result<Option<VipRecord>, RepositoryError> {
        Ok(self.records.lock().iter().find(|r| r.id == id).cloned())
    }

    async fn list(&self) -> Result<Vec<VipRecord>, RepositoryError> {
        Ok(self.records.lock().clone())
    }

    async fn update(&self, record: VipRecord) -> Result<Option<VipRecord>, RepositoryError> {
        let mut records = self.records.lock();
        let Some(existing) = records.iter_mut().find(|r| r.id == record.id) else {
            return Ok(None);
        };
        let updated = VipRecord::new(
            record.id,
            record.customer_name,
            record.user_id,
            record.kind,
            record.roulette_date,
            record.end_date,
            record.status,
            record.note,
            existing.created_at,
            Utc::now(),
        );
        *existing = updated.clone();
        Ok(Some(updated))
    }

    async fn delete(&self, id: VipRecordId) -> Result<bool, RepositoryError> {
        let mut records = self.records.lock();
        let len_before = records.len();
        records.retain(|r| r.id != id);
        Ok(records.len() != len_before)
    }
}
