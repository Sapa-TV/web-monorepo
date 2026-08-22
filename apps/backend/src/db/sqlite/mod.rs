use std::time::Duration;

#[cfg(test)]
use std::path::{Path, PathBuf};

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Error, SqlitePool};

#[cfg(test)]
use tokio::fs;

use crate::error::RepositoryError;

pub mod admin;
pub mod config;
pub mod platform;
pub mod platform_credential;
pub mod session;

pub async fn connect_url(url: &str) -> Result<SqlitePool, RepositoryError> {
    let options: SqliteConnectOptions = url
        .parse()
        .map_err(|e| RepositoryError::Database(format!("invalid sqlite url: {e}")))?;
    open(with_pragmas(options)).await
}

#[cfg(test)]
pub(crate) async fn connect_path(path: &Path) -> Result<SqlitePool, RepositoryError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| RepositoryError::Database(format!("failed to create db dir: {e}")))?;
    }
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    open(with_pragmas(options)).await
}

fn with_pragmas(options: SqliteConnectOptions) -> SqliteConnectOptions {
    options
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5))
}

async fn open(options: SqliteConnectOptions) -> Result<SqlitePool, RepositoryError> {
    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .map_err(map_err)?;
    run_migrations(&pool).await?;
    Ok(pool)
}

async fn run_migrations(pool: &SqlitePool) -> Result<(), RepositoryError> {
    static MIGRATOR: Migrator = sqlx::migrate!();
    MIGRATOR.run(pool).await.map_err(|e| match e {
        MigrateError::Execute(inner) => map_err(inner),
        other => RepositoryError::Database(other.to_string()),
    })
}

pub(crate) fn map_err(e: Error) -> RepositoryError {
    RepositoryError::Database(e.to_string())
}

pub(crate) fn conflict_on_unique(e: Error, message: &str) -> RepositoryError {
    match &e {
        Error::Database(db) if db.is_unique_violation() => {
            RepositoryError::Conflict(message.to_string())
        }
        _ => map_err(e),
    }
}

#[cfg(test)]
#[cfg(test)]
pub(crate) async fn test_pool() -> (SqlitePool, PathBuf) {
    use std::env;

    let path = env::temp_dir().join(format!("sapa-test-{}.db", uuid::Uuid::now_v7().simple()));
    let pool = connect_path(&path)
        .await
        .unwrap_or_else(|e| panic!("failed to open test db: {e}"));
    (pool, path)
}
