use std::ops::Deref;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Notify;

use crate::auth::RateLimiter;
use crate::config::Config;
use crate::fetch::Fetcher;

#[derive(Clone)]
pub struct AppState(Arc<Inner>);

pub struct Inner {
    pub db: SqlitePool,
    pub cfg: Config,
    pub fetcher: Fetcher,
    pub limiter: RateLimiter,
    pub wake_scheduler: Notify,
}

impl AppState {
    pub fn new(db: SqlitePool, cfg: Config) -> anyhow::Result<Self> {
        let fetcher = Fetcher::new(cfg.allow_private_networks)?;
        Ok(Self(Arc::new(Inner {
            db,
            cfg,
            fetcher,
            limiter: RateLimiter::default(),
            wake_scheduler: Notify::new(),
        })))
    }
}

impl Deref for AppState {
    type Target = Inner;
    fn deref(&self) -> &Inner {
        &self.0
    }
}
