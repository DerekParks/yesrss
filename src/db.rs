use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use sqlx::{Connection, Sqlite, SqlitePool, Transaction};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

pub async fn connect(path: &Path) -> anyhow::Result<SqlitePool> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let opts = SqliteConnectOptions::from_str("sqlite://")?
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(15))
        .pragma("temp_store", "memory");
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        // Closes connections still counting a transaction that SQLite already rolled back.
        .after_release(|conn, _| {
            Box::pin(async move {
                conn.ping().await?;
                Ok(!conn.is_in_transaction())
            })
        })
        .connect_with(opts)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

/// Starts a transaction that holds the write lock from its first statement.
pub async fn begin_write(db: &SqlitePool) -> sqlx::Result<Transaction<'static, Sqlite>> {
    db.begin_with("BEGIN IMMEDIATE").await
}

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
