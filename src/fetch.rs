use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::header::{self, HeaderMap, HeaderValue};
use reqwest::redirect::{Attempt, Policy};
use reqwest::{StatusCode, Url};

pub const USER_AGENT: &str = concat!(
    "yesrss/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/DerekParks/yesrss)"
);
const MAX_REDIRECTS: usize = 8;

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("refusing to fetch non-public address {0} (start with --allow-private-networks to permit)")]
    Blocked(String),
    #[error("HTTP {0}")]
    Status(StatusCode, Option<Duration>),
    #[error("response larger than {0} bytes")]
    TooLarge(usize),
    #[error("{}", error_chain(.0))]
    Request(#[from] reqwest::Error),
}

fn error_chain(e: &dyn std::error::Error) -> String {
    let mut msg = e.to_string();
    let mut src = e.source();
    while let Some(s) = src {
        let s_msg = s.to_string();
        if !msg.contains(&s_msg) {
            msg.push_str(": ");
            msg.push_str(&s_msg);
        }
        src = s.source();
    }
    msg
}

impl FetchError {
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::Status(_, ra) => *ra,
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum FetchOutcome {
    NotModified,
    Fetched(Fetched),
}

#[derive(Debug)]
pub struct Fetched {
    pub body: Vec<u8>,
    pub final_url: Url,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[derive(Clone)]
pub struct Fetcher {
    client: reqwest::Client,
    allow_private: bool,
}

impl Fetcher {
    pub fn new(allow_private: bool) -> anyhow::Result<Self> {
        let mut b = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(45))
            .redirect(redirect_policy(allow_private));
        if !allow_private {
            b = b.dns_resolver(Arc::new(PublicOnlyResolver));
        }
        Ok(Self { client: b.build()?, allow_private })
    }

    pub async fn get(
        &self,
        url: &str,
        etag: Option<&str>,
        last_modified: Option<&str>,
        max_bytes: usize,
    ) -> Result<FetchOutcome, FetchError> {
        let url = parse_http_url(url)?;
        if !self.allow_private {
            check_literal_host(&url)?;
        }
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            HeaderValue::from_static(
                "application/atom+xml, application/rss+xml, application/feed+json, application/json;q=0.9, application/xml;q=0.9, text/xml;q=0.9, text/html;q=0.8, */*;q=0.5",
            ),
        );
        if let Some(v) = etag.and_then(|v| HeaderValue::from_str(v).ok()) {
            headers.insert(header::IF_NONE_MATCH, v);
        }
        if let Some(v) = last_modified.and_then(|v| HeaderValue::from_str(v).ok()) {
            headers.insert(header::IF_MODIFIED_SINCE, v);
        }
        let resp = self.client.get(url).headers(headers).send().await?;
        let status = resp.status();
        if status == StatusCode::NOT_MODIFIED {
            return Ok(FetchOutcome::NotModified);
        }
        if !status.is_success() {
            let retry_after = resp
                .headers()
                .get(header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok())
                .map(Duration::from_secs);
            return Err(FetchError::Status(status, retry_after));
        }
        let header_str = |name| {
            resp.headers()
                .get(name)
                .and_then(|v: &HeaderValue| v.to_str().ok())
                .map(str::to_owned)
        };
        let etag = header_str(header::ETAG);
        let last_modified = header_str(header::LAST_MODIFIED);
        let final_url = resp.url().clone();
        if resp.content_length().is_some_and(|n| n as usize > max_bytes) {
            return Err(FetchError::TooLarge(max_bytes));
        }
        let mut body = Vec::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if body.len() + chunk.len() > max_bytes {
                return Err(FetchError::TooLarge(max_bytes));
            }
            body.extend_from_slice(&chunk);
        }
        Ok(FetchOutcome::Fetched(Fetched {
            body,
            final_url,
            etag,
            last_modified,
        }))
    }
}

pub fn parse_http_url(raw: &str) -> Result<Url, FetchError> {
    let url = Url::parse(raw.trim()).map_err(|e| FetchError::InvalidUrl(format!("{raw}: {e}")))?;
    match url.scheme() {
        "http" | "https" if url.host().is_some() => Ok(url),
        _ => Err(FetchError::InvalidUrl(format!("{raw}: only http(s) URLs are supported"))),
    }
}

fn check_literal_host(url: &Url) -> Result<(), FetchError> {
    let ip = match url.host() {
        Some(url::Host::Ipv4(ip)) => IpAddr::V4(ip),
        Some(url::Host::Ipv6(ip)) => IpAddr::V6(ip),
        Some(url::Host::Domain(d)) if d.eq_ignore_ascii_case("localhost") || d.ends_with(".localhost") => {
            return Err(FetchError::Blocked(d.to_owned()));
        }
        _ => return Ok(()),
    };
    if is_public_ip(ip) {
        Ok(())
    } else {
        Err(FetchError::Blocked(ip.to_string()))
    }
}

fn redirect_policy(allow_private: bool) -> Policy {
    Policy::custom(move |attempt: Attempt| {
        if attempt.previous().len() >= MAX_REDIRECTS {
            return attempt.error("too many redirects");
        }
        let url = attempt.url();
        if !matches!(url.scheme(), "http" | "https") {
            let msg = format!("redirect to unsupported scheme {}", url.scheme());
            return attempt.error(msg);
        }
        if !allow_private && let Err(e) = check_literal_host(url) {
            return attempt.error(e);
        }
        attempt.follow()
    })
}

struct PublicOnlyResolver;

impl Resolve for PublicOnlyResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
                .await?
                .filter(|a| is_public_ip(a.ip()))
                .collect();
            if addrs.is_empty() {
                return Err(Box::new(FetchError::Blocked(host)) as Box<dyn std::error::Error + Send + Sync>);
            }
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => is_public_v6(v6),
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_multicast()
        || a == 0
        || a >= 240
        || (a == 100 && (64..128).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 198 && (18..20).contains(&b)))
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_public_v4(v4);
    }
    let seg = ip.segments();
    if seg[0] == 0x64 && seg[1] == 0xff9b && seg[2..6] == [0, 0, 0, 0] {
        let v4 = Ipv4Addr::new((seg[6] >> 8) as u8, seg[6] as u8, (seg[7] >> 8) as u8, seg[7] as u8);
        return is_public_v4(v4);
    }
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || ip.is_unique_local()
        || ip.is_unicast_link_local()
        || (seg[0] == 0x2001 && seg[1] == 0x0db8)
        || seg[0..6] == [0, 0, 0, 0, 0, 0])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn public(s: &str) -> bool {
        is_public_ip(s.parse().unwrap())
    }

    #[test]
    fn classifies_addresses() {
        for ip in [
            "127.0.0.1", "10.1.2.3", "172.16.0.1", "192.168.1.1", "169.254.169.254", "0.0.0.0",
            "100.64.0.1", "255.255.255.255", "::1", "::", "fe80::1", "fd00::1", "::ffff:192.168.0.1",
            "64:ff9b::a00:1", "::127.0.0.1", "2001:db8::1",
        ] {
            assert!(!public(ip), "{ip} should be private");
        }
        for ip in ["1.1.1.1", "93.184.216.34", "2606:4700:4700::1111", "::ffff:8.8.8.8", "64:ff9b::808:808"] {
            assert!(public(ip), "{ip} should be public");
        }
    }

    #[test]
    fn rejects_bad_urls() {
        assert!(parse_http_url("file:///etc/passwd").is_err());
        assert!(parse_http_url("gopher://x").is_err());
        assert!(parse_http_url("not a url").is_err());
        assert!(parse_http_url("https://example.com/feed").is_ok());
    }

    #[tokio::test]
    async fn blocks_private_targets() {
        let f = Fetcher::new(false).unwrap();
        for url in ["http://127.0.0.1:1/", "http://[::1]:1/", "http://localhost:1/", "http://169.254.169.254/latest"] {
            let err = f.get(url, None, None, 1024).await.unwrap_err();
            assert!(matches!(err, FetchError::Blocked(_)), "{url}: {err}");
        }
    }
}
