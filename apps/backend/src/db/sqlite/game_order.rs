use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;

use crate::db::sqlite::map_err;
use crate::error::RepositoryError;
use crate::orders::game::{GameOrder, GameOrderId, GameOrderKind, NewGameOrder, OrderSource};
use crate::orders::repository::GameOrderRepository;
use crate::orders::status::OrderStatus;
use crate::user::UserId;

#[non_exhaustive]
pub struct SqliteGameOrderRepository {
    pool: SqlitePool,
}

impl SqliteGameOrderRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn kind_from_db(value: &str) -> Result<GameOrderKind, RepositoryError> {
    value
        .parse()
        .map_err(|_| RepositoryError::Database(format!("invalid game order kind: {value}")))
}

fn source_from_db(value: &str) -> Result<OrderSource, RepositoryError> {
    value
        .parse()
        .map_err(|_| RepositoryError::Database(format!("invalid order source: {value}")))
}

fn status_from_db(value: &str) -> Result<OrderStatus, RepositoryError> {
    value
        .parse()
        .map_err(|_| RepositoryError::Database(format!("invalid order status: {value}")))
}

#[derive(sqlx::FromRow)]
struct GameOrderRow {
    id: i64,
    title: Option<String>,
    customer_name: String,
    user_id: Option<i64>,
    kind: String,
    source: String,
    status: String,
    completed_at: Option<NaiveDate>,
    comment: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl GameOrderRow {
    fn into_order(self) -> Result<GameOrder, RepositoryError> {
        Ok(GameOrder::new(
            GameOrderId::new(self.id as u32),
            self.title,
            self.customer_name,
            self.user_id.map(|id| UserId::new(id as u32)),
            kind_from_db(&self.kind)?,
            source_from_db(&self.source)?,
            status_from_db(&self.status)?,
            self.completed_at,
            self.comment,
            self.created_at,
            self.updated_at,
        ))
    }
}

impl GameOrderRepository for SqliteGameOrderRepository {
    async fn create(&self, order: NewGameOrder) -> Result<GameOrder, RepositoryError> {
        let now = Utc::now();
        let user_id = order.user_id.map(|id| id.get() as i64);
        let kind: &'static str = (&order.kind).into();
        let source: &'static str = (&order.source).into();
        let status: &'static str = (&order.status).into();
        let row = sqlx::query!(
            "INSERT INTO game_orders (title, customer_name, user_id, kind, source, status, completed_at, comment, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id AS \"id!: i64\"",
            order.title,
            order.customer_name,
            user_id,
            kind,
            source,
            status,
            order.completed_at,
            order.comment,
            now,
            now
        )
        .fetch_one(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(GameOrder::new(
            GameOrderId::new(row.id as u32),
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
        ))
    }

    async fn get_by_id(&self, id: GameOrderId) -> Result<Option<GameOrder>, RepositoryError> {
        let row = sqlx::query_as!(
            GameOrderRow,
            "SELECT id AS \"id!: i64\", title, customer_name, user_id AS \"user_id?: i64\", kind, source, status, completed_at AS \"completed_at?: NaiveDate\", comment, created_at AS \"created_at: DateTime<Utc>\", updated_at AS \"updated_at: DateTime<Utc>\"
             FROM game_orders WHERE id = ?",
            id.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        row.map(GameOrderRow::into_order).transpose()
    }

    async fn list(&self) -> Result<Vec<GameOrder>, RepositoryError> {
        let rows = sqlx::query_as!(
            GameOrderRow,
            "SELECT id AS \"id!: i64\", title, customer_name, user_id AS \"user_id?: i64\", kind, source, status, completed_at AS \"completed_at?: NaiveDate\", comment, created_at AS \"created_at: DateTime<Utc>\", updated_at AS \"updated_at: DateTime<Utc>\"
             FROM game_orders ORDER BY id"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_err)?;
        rows.into_iter().map(GameOrderRow::into_order).collect()
    }

    async fn update(&self, order: GameOrder) -> Result<Option<GameOrder>, RepositoryError> {
        let now = Utc::now();
        let user_id = order.user_id.map(|id| id.get() as i64);
        let kind: &'static str = (&order.kind).into();
        let source: &'static str = (&order.source).into();
        let status: &'static str = (&order.status).into();
        let row = sqlx::query!(
            "UPDATE game_orders SET title = ?1, customer_name = ?2, user_id = ?3, kind = ?4, source = ?5, status = ?6, completed_at = ?7, comment = ?8, updated_at = ?9
             WHERE id = ?10
             RETURNING id AS \"id!: i64\"",
            order.title,
            order.customer_name,
            user_id,
            kind,
            source,
            status,
            order.completed_at,
            order.comment,
            now,
            order.id.get()
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_err)?;
        Ok(row.map(|_| {
            GameOrder::new(
                order.id,
                order.title.clone(),
                order.customer_name.clone(),
                order.user_id,
                order.kind,
                order.source,
                order.status,
                order.completed_at,
                order.comment.clone(),
                order.created_at,
                now,
            )
        }))
    }

    async fn delete(&self, id: GameOrderId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!("DELETE FROM game_orders WHERE id = ?", id.get())
            .execute(&self.pool)
            .await
            .map_err(map_err)?;
        Ok(result.rows_affected() > 0)
    }
}
