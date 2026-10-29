use std::ops::Deref;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::Notify;

use crate::atproto::Atproto;
use crate::atproto::identity::Network;
use crate::auth::RateLimiter;
use crate::config::Config;
use crate::fetch::Fetcher;
use crate::links::LinkRules;

#[derive(Clone)]
pub struct AppState(Arc<Inner>);

pub struct Inner {
    pub db: SqlitePool,
    pub cfg: Config,
    pub fetcher: Fetcher,
    pub limiter: RateLimiter,
    pub wake_scheduler: Notify,
    pub atproto: Option<Atproto>,
    pub link_rules: LinkRules,
}

impl AppState {
    pub fn new(db: SqlitePool, cfg: Config) -> anyhow::Result<Self> {
        Self::with_network(db, cfg, Network::default())
    }

    pub fn with_network(db: SqlitePool, cfg: Config, net: Network) -> anyhow::Result<Self> {
        let fetcher = Fetcher::new(cfg.allow_private_networks)?;
        let atproto = cfg.public_url.clone().map(|u| Atproto::new(u, net));
        let link_rules = LinkRules::load(cfg.link_rules.as_deref())?;
        Ok(Self(Arc::new(Inner {
            db,
            cfg,
            fetcher,
            limiter: RateLimiter::default(),
            wake_scheduler: Notify::new(),
            atproto,
            link_rules,
        })))
    }
}

impl Deref for AppState {
    type Target = Inner;
    fn deref(&self) -> &Inner {
        &self.0
    }
}
