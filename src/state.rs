use std::ops::Deref;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Notify;

use crate::atproto::Atproto;
use crate::atproto::identity::Network;
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
    pub atproto: Option<Atproto>,
}

impl AppState {
    pub fn new(db: SqlitePool, cfg: Config) -> anyhow::Result<Self> {
        Self::with_network(db, cfg, Network::default())
    }

    pub fn with_network(db: SqlitePool, cfg: Config, net: Network) -> anyhow::Result<Self> {
        let fetcher = Fetcher::new(cfg.allow_private_networks)?;
        let atproto = cfg.public_url.clone().map(|u| Atproto::new(u, net));
        Ok(Self(Arc::new(Inner {
            db,
            cfg,
            fetcher,
            limiter: RateLimiter::default(),
            wake_scheduler: Notify::new(),
            atproto,
        })))
    }
}

impl Deref for AppState {
    type Target = Inner;
    fn deref(&self) -> &Inner {
        &self.0
    }
}
