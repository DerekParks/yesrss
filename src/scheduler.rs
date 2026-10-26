use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::db::now;
use crate::error::AppError;
use crate::feeds::{purge_old_entries, refresh_feed};
use crate::state::AppState;

const MAX_IDLE: Duration = Duration::from_secs(60);
const BATCH: i64 = 200;
const PURGE_EVERY: Duration = Duration::from_secs(6 * 60 * 60);

pub fn spawn(state: AppState) {
    tokio::spawn(run(state));
}

async fn run(state: AppState) {
    let permits = Arc::new(Semaphore::new(state.cfg.fetch_concurrency.max(1)));
    let mut last_purge: Option<tokio::time::Instant> = None;
    loop {
        if last_purge.is_none_or(|t| t.elapsed() >= PURGE_EVERY) {
            match purge_old_entries(&state).await {
                Ok(n) if n > 0 => tracing::info!(deleted = n, "purged old entries"),
                Ok(_) => {}
                Err(e) => tracing::error!(error = %e, "purge failed"),
            }
            last_purge = Some(tokio::time::Instant::now());
        }

        match refresh_due(&state, &permits).await {
            Ok(n) if n as i64 == BATCH => continue,
            Ok(_) => {}
            Err(e) => tracing::error!(error = %e, "scheduler query failed"),
        }

        let wait = next_due_in(&state).await.unwrap_or(MAX_IDLE).min(MAX_IDLE);
        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            _ = state.wake_scheduler.notified() => {}
        }
    }
}

async fn refresh_due(state: &AppState, permits: &Arc<Semaphore>) -> anyhow::Result<usize> {
    let due: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM feeds WHERE next_fetch_at <= ? ORDER BY next_fetch_at LIMIT ?",
    )
    .bind(now())
    .bind(BATCH)
    .fetch_all(&state.db)
    .await?;
    let mut tasks = JoinSet::new();
    for feed_id in due.iter().copied() {
        let permit = permits.clone().acquire_owned().await?;
        let st = state.clone();
        tasks.spawn(async move {
            let _permit = permit;
            match refresh_feed(&st, feed_id).await {
                Ok(n) if n > 0 => tracing::info!(feed_id, new_entries = n, "feed refreshed"),
                Ok(_) => tracing::debug!(feed_id, "feed unchanged"),
                Err(AppError::BadRequest(_)) => {}
                Err(e) => tracing::error!(feed_id, error = %e, "feed refresh failed"),
            }
        });
    }
    while tasks.join_next().await.is_some() {}
    Ok(due.len())
}

async fn next_due_in(state: &AppState) -> Option<Duration> {
    let next: Option<i64> = sqlx::query_scalar("SELECT MIN(next_fetch_at) FROM feeds")
        .fetch_one(&state.db)
        .await
        .ok()
        .flatten();
    next.map(|t| Duration::from_secs((t - now()).max(1) as u64))
}
