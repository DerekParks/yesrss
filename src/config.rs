use std::net::SocketAddr;

use url::Url;

#[derive(clap::Args, Clone, Debug)]
pub struct Config {
    /// Address to listen on.
    #[arg(long, env = "YESRSS_LISTEN", default_value = "127.0.0.1:8080")]
    pub listen: SocketAddr,

    /// Mark session cookies `Secure`. Enable when served over HTTPS (e.g. behind a TLS proxy).
    #[arg(long, env = "YESRSS_SECURE_COOKIES")]
    pub secure_cookies: bool,

    /// Allow fetching feeds from loopback, private, and link-local addresses.
    /// Off by default so the fetcher cannot be used to probe your LAN.
    #[arg(long, env = "YESRSS_ALLOW_PRIVATE_NETWORKS")]
    pub allow_private_networks: bool,

    /// Minutes between refreshes of a healthy feed.
    #[arg(long, env = "YESRSS_REFRESH_MINUTES", default_value_t = 30)]
    pub refresh_minutes: u64,

    /// Days to keep read, unstarred entries that have dropped out of their feed.
    #[arg(long, env = "YESRSS_RETENTION_DAYS", default_value_t = 90)]
    pub retention_days: u64,

    /// Maximum number of feeds fetched at the same time.
    #[arg(long, env = "YESRSS_FETCH_CONCURRENCY", default_value_t = 8)]
    pub fetch_concurrency: usize,

    /// Public HTTPS URL of this server. Required for atproto (Bluesky) sign-in, whose
    /// authorization server must fetch this server's OAuth client metadata.
    #[arg(long, env = "YESRSS_PUBLIC_URL")]
    pub public_url: Option<Url>,

    /// Largest feed document accepted, in bytes.
    #[arg(long, env = "YESRSS_MAX_FEED_BYTES", default_value_t = 16 * 1024 * 1024)]
    pub max_feed_bytes: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:8080".parse().unwrap(),
            secure_cookies: false,
            allow_private_networks: false,
            refresh_minutes: 30,
            retention_days: 90,
            fetch_concurrency: 8,
            public_url: None,
            max_feed_bytes: 16 * 1024 * 1024,
        }
    }
}
