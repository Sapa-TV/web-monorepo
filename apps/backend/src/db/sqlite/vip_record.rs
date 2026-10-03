use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;

use crate::db::sqlite::map_err;
use crate::error::RepositoryError;
use crate::orders::repository::VipRecordRepository;
use crate::orders::vip::{NewVipRecord, VipKind, VipRecord, VipRecordId, VipStatus};
use crate::user::UserId;

#[non_exhaustive]
pub struct SqliteVipRecordRepository {
    pool: SqlitePool,
}

impl SqliteVipRecordRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn kind_from_db(value: &str) -> Result<VipKind, RepositoryError> {
    value
        .parse()
        .map_err(|_| RepositoryError::Database(format!("invalid vip kind: {value}")))
}

fn status_from_db(value: &str) -> Result<VipStatus, RepositoryError> {
    value
        .parse()
        .map_err(|_| RepositoryError::Database(format!("invalid vip status: {value}")))
}

#[derive(sqlx::FromRow)]
struct VipRecordRow {
    id: i64,
    customer_name: String,
    user_id: Option<i64>,
    kind: String,
    roulette_date: NaiveDate,
    end_date: NaiveDate,
    status: String,
    note: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl VipRecordRow {
    fn into_record(self) -> Result<VipRecord, RepositoryError> {
        Ok(VipRecord::new(
            VipRecordId::new(self.id as u32),
            self.customer_name,
            self.user_id.map(|id| UserId::new(id as u32)),
            kind_from_db(&self.kind)?,
            self.roulette_date,
            self.end_date,
            status_from_db(&self.status)?,
            self.note,
            self.created_at,
            self.updated_at,
        ))
    }
}

impl VipRecordRepository for SqliteVipRecordRepository {
    async fn create(
        &self,
        record: NewVipRecord,
        end_date: NaiveDate,
    ) -> Result<VipRecord, RepositoryError> {
        let now = Utc::now();
        let user_id = record.user_id.map(|id| id.get() as i64);
        let kind: &'static str = (&record.kind).into();
        let status: &'static str = (&VipStatus::Active).into();
        let row = sqlx::query!(
            "INSERT INTO vip_records (customer_name, user_id, kind, roulette_date, end_date, status, note, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id AS \"id!: i64\"",
            record.customer_name,
            user_id,
            kind,
            record.roulette_date,
            end_date,
            status,
            record.note,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(VipRecord::new(
            VipRecordId::new(row.id as u32),
            record.customer_name,
            record.user_id,
            record.kind,
            record.roulette_date,
            end_date,
            VipStatus::Active,
            record.note,
            now,
            now,
        ))
    }

    async fn get_by_id(&self, id: VipRecordId) -> Result<Option<VipRecord>, RepositoryError> {
        let row = sqlx::query_as!(
            VipRecordRow,
            "SELECT id AS \"id!: i64\", customer_name, user_id AS \"user_id?: i64\", kind, roulette_date AS \"roulette_date: NaiveDate\", end_date AS \"end_date: NaiveDate\", status, note, created_at AS \"created_at: DateTime<Utc>\", updated_at AS \"updated_at: DateTime<Utc>\"
             FROM vip_records WHERE id = ?",
            id.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        row.map(VipRecordRow::into_record).transpose()
    }

    async fn list(&self) -> Result<Vec<VipRecord>, RepositoryError> {
        let rows = sqlx::query_as!(
            VipRecordRow,
            "SELECT id AS \"id!: i64\", customer_name, user_id AS \"user_id?: i64\", kind, roulette_date AS \"roulette_date: NaiveDate\", end_date AS \"end_date: NaiveDate\", status, note, created_at AS \"created_at: DateTime<Utc>\", updated_at AS \"updated_at: DateTime<Utc>\"
             FROM vip_records ORDER BY id"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        rows.into_iter().map(VipRecordRow::into_record).collect()
    }

    async fn update(&self, record: VipRecord) -> Result<Option<VipRecord>, RepositoryError> {
        let now = Utc::now();
        let user_id = record.user_id.map(|id| id.get() as i64);
        let kind: &'static str = (&record.kind).into();
        let status: &'static str = (&record.status).into();
        let row = sqlx::query!(
            "UPDATE vip_records SET customer_name = ?1, user_id = ?2, kind = ?3, roulette_date = ?4, end_date = ?5, status = ?6, note = ?7, updated_at = ?8
             WHERE id = ?9
             RETURNING id AS \"id!: i64\"",
            record.customer_name,
            user_id,
            kind,
            record.roulette_date,
            record.end_date,
            status,
            record.note,
            now,
            record.id.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|_| {
            VipRecord::new(
                record.id,
                record.customer_name.clone(),
                record.user_id,
                record.kind,
                record.roulette_date,
                record.end_date,
                record.status,
                record.note.clone(),
                record.created_at,
                now,
            )
        }))
    }

    async fn delete(&self, id: VipRecordId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!("DELETE FROM vip_records WHERE id = ?", id.get())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() > 0)
    }
}
