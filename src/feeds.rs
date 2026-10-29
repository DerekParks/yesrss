use std::time::Duration;

use serde::Serialize;
use sqlx::SqlitePool;
use url::Url;

use crate::db::{begin_write, now};
use crate::error::{AppError, AppResult};
use crate::fetch::{FetchOutcome, Fetched, Fetcher, parse_http_url};
use crate::opml::OpmlFeed;
use crate::parse::{NewEntry, ParsedFeed, parse_feed};
use crate::state::AppState;

const MAX_BACKOFF: Duration = Duration::from_secs(24 * 60 * 60);
const ICON_MAX_BYTES: usize = 512 * 1024;
const PAGE_MAX_BYTES: usize = 4 * 1024 * 1024;
const ICON_REFRESH_SECS: i64 = 30 * 24 * 60 * 60;
const COMMON_FEED_PATHS: &[&str] = &[
    "/feed", "/rss", "/feed.xml", "/rss.xml", "/atom.xml", "/index.xml", "/feed.json", "/rss/",
];

#[derive(Debug, Serialize)]
pub struct Candidate {
    pub url: String,
    pub title: Option<String>,
}

struct Resolved {
    url: Url,
    fetched: Fetched,
    parsed: ParsedFeed,
}

fn normalize_input(input: &str) -> AppResult<Url> {
    let input = input.trim();
    let with_scheme = if input.contains("://") {
        input.to_owned()
    } else {
        format!("https://{input}")
    };
    parse_http_url(&with_scheme).map_err(|e| AppError::bad(e.to_string()))
}

async fn fetch_fresh(fetcher: &Fetcher, url: &Url, max: usize) -> AppResult<Fetched> {
    match fetcher.get(url.as_str(), None, None, max).await {
        Ok(FetchOutcome::Fetched(f)) => Ok(f),
        Ok(FetchOutcome::NotModified) => Err(AppError::bad("unexpected 304 response")),
        Err(e) => Err(AppError::bad(format!("could not fetch {url}: {e}"))),
    }
}

fn alternate_links(html: &[u8], base: &Url) -> Vec<Candidate> {
    let doc = scraper::Html::parse_document(&String::from_utf8_lossy(html));
    let sel = scraper::Selector::parse("link[href]").expect("valid selector");
    doc.select(&sel)
        .filter(|el| {
            let v = el.value();
            let rel = v.attr("rel").unwrap_or("").to_ascii_lowercase();
            let ty = v.attr("type").unwrap_or("").to_ascii_lowercase();
            rel.split_whitespace().any(|r| r == "alternate")
                && (ty.contains("rss") || ty.contains("atom") || ty.contains("feed+json"))
        })
        .filter_map(|el| {
            let url = base.join(el.value().attr("href")?).ok()?;
            matches!(url.scheme(), "http" | "https").then(|| Candidate {
                url: url.into(),
                title: el.value().attr("title").map(str::to_owned),
            })
        })
        .collect()
}

/// Finds feeds for `input`, which may be a feed URL, a web page, or a bare domain.
pub async fn discover(state: &AppState, input: &str) -> AppResult<Vec<Candidate>> {
    let url = normalize_input(input)?;
    let max = state.cfg.max_feed_bytes;
    let fetched = fetch_fresh(&state.fetcher, &url, max).await?;
    if let Ok(parsed) = parse_feed(&fetched.body, &fetched.final_url, now(), &state.link_rules) {
        return Ok(vec![Candidate { url: fetched.final_url.into(), title: Some(parsed.title) }]);
    }
    let mut found = alternate_links(&fetched.body, &fetched.final_url);
    found.dedup_by(|a, b| a.url == b.url);
    if !found.is_empty() {
        return Ok(found);
    }
    for path in COMMON_FEED_PATHS {
        let Ok(guess) = fetched.final_url.join(path) else { continue };
        if let Ok(f) = fetch_fresh(&state.fetcher, &guess, max).await
            && let Ok(parsed) = parse_feed(&f.body, &f.final_url, now(), &state.link_rules)
        {
            return Ok(vec![Candidate { url: f.final_url.into(), title: Some(parsed.title) }]);
        }
    }
    Err(AppError::bad(format!("no feed found at {url}")))
}

async fn resolve(state: &AppState, input: &str) -> AppResult<Resolved> {
    let url = normalize_input(input)?;
    let max = state.cfg.max_feed_bytes;
    let fetched = fetch_fresh(&state.fetcher, &url, max).await?;
    if let Ok(parsed) = parse_feed(&fetched.body, &fetched.final_url, now(), &state.link_rules) {
        return Ok(Resolved { url: fetched.final_url.clone(), fetched, parsed });
    }
    let candidates = discover(state, input).await?;
    let first = candidates.first().ok_or_else(|| AppError::bad("no feed found"))?;
    let url = Url::parse(&first.url).map_err(|e| AppError::bad(e.to_string()))?;
    let fetched = fetch_fresh(&state.fetcher, &url, max).await?;
    let parsed = parse_feed(&fetched.body, &fetched.final_url, now(), &state.link_rules)
        .map_err(|e| AppError::bad(format!("{url} is not a valid feed: {e}")))?;
    Ok(Resolved { url: fetched.final_url.clone(), fetched, parsed })
}

pub async fn folder_by_name(db: &SqlitePool, user_id: i64, name: &str) -> AppResult<i64> {
    let name = name.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(AppError::bad("folder name must be 1-200 characters"));
    }
    Ok(sqlx::query_scalar(
        "INSERT INTO folders (user_id, name) VALUES (?, ?)
         ON CONFLICT (user_id, name) DO UPDATE SET name = excluded.name RETURNING id",
    )
    .bind(user_id)
    .bind(name)
    .fetch_one(db)
    .await?)
}

pub async fn check_folder(db: &SqlitePool, user_id: i64, folder_id: Option<i64>) -> AppResult<()> {
    if let Some(id) = folder_id {
        sqlx::query_scalar::<_, i64>("SELECT id FROM folders WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::bad("unknown folder"))?;
    }
    Ok(())
}

/// Subscribes `user_id` to the feed at `input` and stores its current entries.
pub async fn subscribe(
    state: &AppState,
    user_id: i64,
    input: &str,
    folder_id: Option<i64>,
    title: Option<&str>,
) -> AppResult<i64> {
    check_folder(&state.db, user_id, folder_id).await?;
    let r = resolve(state, input).await?;
    let t = now();
    let user_title = title.map(str::trim).filter(|s| !s.is_empty());
    let feed_id: i64 = sqlx::query_scalar(
        "INSERT INTO feeds (user_id, folder_id, url, site_url, title, user_title, etag, last_modified,
                            last_fetched_at, last_success_at, next_fetch_at, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(user_id)
    .bind(folder_id)
    .bind(r.url.as_str())
    .bind(&r.parsed.site_url)
    .bind(&r.parsed.title)
    .bind(user_title)
    .bind(&r.fetched.etag)
    .bind(&r.fetched.last_modified)
    .bind(t)
    .bind(t)
    .bind(next_fetch(state, t))
    .bind(t)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match AppError::from(e) {
        AppError::Conflict(_) => AppError::Conflict("already subscribed to this feed".into()),
        other => other,
    })?;
    store_entries(&state.db, feed_id, user_id, &r.parsed.entries, t, None).await?;
    let st = state.clone();
    tokio::spawn(async move { update_favicon(&st, feed_id).await });
    Ok(feed_id)
}

pub async fn store_entries(
    db: &SqlitePool,
    feed_id: i64,
    user_id: i64,
    entries: &[NewEntry],
    seen_at: i64,
    read_before: Option<i64>,
) -> AppResult<usize> {
    let mut tx = begin_write(db).await?;
    let mut added = 0;
    for e in entries {
        let existing: Option<(i64, String)> =
            sqlx::query_as("SELECT id, content_hash FROM entries WHERE feed_id = ? AND guid = ?")
                .bind(feed_id)
                .bind(&e.guid)
                .fetch_optional(&mut *tx)
                .await?;
        match existing {
            None => {
                let backfilled_read = read_before.is_some_and(|t| e.published_at <= t);
                let id: i64 = sqlx::query_scalar(
                    "INSERT INTO entries (feed_id, user_id, guid, url, title, author, content, snippet,
                         enclosure_url, enclosure_type, content_hash, published_at, created_at, last_seen_at,
                         is_read, read_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
                )
                .bind(feed_id)
                .bind(user_id)
                .bind(&e.guid)
                .bind(&e.url)
                .bind(&e.title)
                .bind(&e.author)
                .bind(&e.content)
                .bind(&e.snippet)
                .bind(&e.enclosure_url)
                .bind(&e.enclosure_type)
                .bind(&e.content_hash)
                .bind(e.published_at)
                .bind(seen_at)
                .bind(seen_at)
                .bind(backfilled_read)
                .bind(backfilled_read.then_some(seen_at))
                .fetch_one(&mut *tx)
                .await?;
                sqlx::query("INSERT INTO entries_fts (rowid, title, body) VALUES (?, ?, ?)")
                    .bind(id)
                    .bind(&e.title)
                    .bind(&e.search_text)
                    .execute(&mut *tx)
                    .await?;
                added += 1;
            }
            Some((id, hash)) if hash != e.content_hash => {
                sqlx::query(
                    "UPDATE entries SET url = ?, title = ?, author = ?, content = ?, snippet = ?,
                         enclosure_url = ?, enclosure_type = ?, content_hash = ?, last_seen_at = ?
                     WHERE id = ?",
                )
                .bind(&e.url)
                .bind(&e.title)
                .bind(&e.author)
                .bind(&e.content)
                .bind(&e.snippet)
                .bind(&e.enclosure_url)
                .bind(&e.enclosure_type)
                .bind(&e.content_hash)
                .bind(seen_at)
                .bind(id)
                .execute(&mut *tx)
                .await?;
                sqlx::query("UPDATE entries_fts SET title = ?, body = ? WHERE rowid = ?")
                    .bind(&e.title)
                    .bind(&e.search_text)
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            }
            Some((id, _)) => {
                sqlx::query("UPDATE entries SET last_seen_at = ? WHERE id = ?")
                    .bind(seen_at)
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }
    tx.commit().await?;
    Ok(added)
}

fn next_fetch(state: &AppState, from: i64) -> i64 {
    let interval = (state.cfg.refresh_minutes.max(1) * 60) as i64;
    from + interval + rand::random_range(0..=interval / 10)
}

fn backoff(state: &AppState, errors: i64, retry_after: Option<Duration>) -> Duration {
    let base = Duration::from_secs(state.cfg.refresh_minutes.max(1) * 60);
    let exp = base.saturating_mul(1 << errors.clamp(0, 6) as u32).min(MAX_BACKOFF);
    retry_after.map_or(exp, |ra| ra.min(MAX_BACKOFF).max(exp))
}

#[derive(sqlx::FromRow)]
struct FeedFetchRow {
    user_id: i64,
    url: String,
    etag: Option<String>,
    last_modified: Option<String>,
    error_count: i64,
    read_before: Option<i64>,
}

/// Fetches one feed and stores any new entries. Returns the number of new entries.
pub async fn refresh_feed(state: &AppState, feed_id: i64) -> AppResult<usize> {
    let feed: FeedFetchRow = sqlx::query_as(
        "SELECT user_id, url, etag, last_modified, error_count,
             CASE WHEN last_success_at IS NULL THEN read_before END AS read_before
         FROM feeds WHERE id = ?",
    )
    .bind(feed_id)
    .fetch_one(&state.db)
    .await?;
    let t = now();
    let result = state
        .fetcher
        .get(&feed.url, feed.etag.as_deref(), feed.last_modified.as_deref(), state.cfg.max_feed_bytes)
        .await;
    let fetched = match result {
        Ok(FetchOutcome::NotModified) => {
            sqlx::query(
                "UPDATE feeds SET last_fetched_at = ?, next_fetch_at = ?, error_count = 0, last_error = NULL
                 WHERE id = ?",
            )
            .bind(t)
            .bind(next_fetch(state, t))
            .bind(feed_id)
            .execute(&state.db)
            .await?;
            return Ok(0);
        }
        Ok(FetchOutcome::Fetched(f)) => f,
        Err(e) => {
            record_failure(state, feed_id, feed.error_count, &e.to_string(), e.retry_after(), t).await?;
            return Err(AppError::bad(e.to_string()));
        }
    };
    let parsed = match parse_feed(&fetched.body, &fetched.final_url, t, &state.link_rules) {
        Ok(p) => p,
        Err(e) => {
            let msg = format!("not a valid feed: {e}");
            record_failure(state, feed_id, feed.error_count, &msg, None, t).await?;
            return Err(AppError::bad(msg));
        }
    };
    match store_refresh(state, feed_id, &feed, &fetched, &parsed, t).await {
        Ok(added) => {
            update_favicon(state, feed_id).await;
            Ok(added)
        }
        Err(e) => {
            let msg = format!("storing entries failed: {e}");
            record_failure(state, feed_id, feed.error_count, &msg, None, t).await?;
            Err(e)
        }
    }
}

async fn store_refresh(
    state: &AppState,
    feed_id: i64,
    feed: &FeedFetchRow,
    fetched: &Fetched,
    parsed: &ParsedFeed,
    t: i64,
) -> AppResult<usize> {
    let added = store_entries(&state.db, feed_id, feed.user_id, &parsed.entries, t, feed.read_before).await?;
    sqlx::query(
        "UPDATE feeds SET user_title = COALESCE(user_title,
                 CASE WHEN last_success_at IS NULL AND title NOT IN (?1, url) THEN title END),
             title = ?1, site_url = COALESCE(?2, site_url), etag = ?3, last_modified = ?4,
             last_fetched_at = ?5, last_success_at = ?5, next_fetch_at = ?6, error_count = 0, last_error = NULL
         WHERE id = ?7",
    )
    .bind(&parsed.title)
    .bind(&parsed.site_url)
    .bind(&fetched.etag)
    .bind(&fetched.last_modified)
    .bind(t)
    .bind(next_fetch(state, t))
    .bind(feed_id)
    .execute(&state.db)
    .await?;
    Ok(added)
}

async fn record_failure(
    state: &AppState,
    feed_id: i64,
    prev_errors: i64,
    msg: &str,
    retry_after: Option<Duration>,
    t: i64,
) -> AppResult<()> {
    let wait = backoff(state, prev_errors, retry_after);
    tracing::warn!(feed_id, error = msg, retry_in_secs = wait.as_secs(), "feed refresh failed");
    sqlx::query(
        "UPDATE feeds SET last_fetched_at = ?, next_fetch_at = ?, error_count = error_count + 1, last_error = ?
         WHERE id = ?",
    )
    .bind(t)
    .bind(t + wait.as_secs() as i64)
    .bind(msg)
    .bind(feed_id)
    .execute(&state.db)
    .await?;
    Ok(())
}

pub fn sniff_image(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => Some("image/png"),
        [0, 0, 1, 0, ..] => Some("image/x-icon"),
        [b'G', b'I', b'F', b'8', ..] => Some("image/gif"),
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("image/webp"),
        _ => None,
    }
}

fn icon_links(html: &[u8], base: &Url) -> Vec<Url> {
    let doc = scraper::Html::parse_document(&String::from_utf8_lossy(html));
    let sel = scraper::Selector::parse("link[href][rel]").expect("valid selector");
    let mut icons: Vec<(u8, Url)> = doc
        .select(&sel)
        .filter_map(|el| {
            let rel = el.value().attr("rel")?.to_ascii_lowercase();
            let rank = if rel.split_whitespace().any(|r| r == "icon") {
                0
            } else if rel.contains("apple-touch-icon") {
                1
            } else {
                return None;
            };
            let url = base.join(el.value().attr("href")?).ok()?;
            matches!(url.scheme(), "http" | "https").then_some((rank, url))
        })
        .collect();
    icons.sort_by_key(|(rank, _)| *rank);
    icons.into_iter().map(|(_, u)| u).collect()
}

async fn find_icon(fetcher: &Fetcher, site: &Url) -> Option<(&'static str, Vec<u8>)> {
    let mut candidates = Vec::new();
    if let Ok(FetchOutcome::Fetched(page)) = fetcher.get(site.as_str(), None, None, PAGE_MAX_BYTES).await {
        candidates = icon_links(&page.body, &page.final_url);
    }
    candidates.extend(site.join("/favicon.ico").ok());
    for url in candidates.into_iter().take(5) {
        if let Ok(FetchOutcome::Fetched(f)) = fetcher.get(url.as_str(), None, None, ICON_MAX_BYTES).await
            && let Some(ty) = sniff_image(&f.body)
        {
            return Some((ty, f.body));
        }
    }
    None
}

#[derive(sqlx::FromRow)]
struct IconTarget {
    site_url: Option<String>,
    url: String,
    fetched_at: Option<i64>,
}

pub async fn update_favicon(state: &AppState, feed_id: i64) {
    let row = sqlx::query_as::<_, IconTarget>(
        "SELECT f.site_url, f.url, i.fetched_at FROM feeds f LEFT JOIN favicons i ON i.feed_id = f.id
         WHERE f.id = ?",
    )
    .bind(feed_id)
    .fetch_optional(&state.db)
    .await;
    let Ok(Some(IconTarget { site_url, url: feed_url, fetched_at })) = row else { return };
    let t = now();
    if fetched_at.is_some_and(|at| t - at < ICON_REFRESH_SECS) {
        return;
    }
    let Some(site) = Url::parse(site_url.as_deref().unwrap_or(&feed_url))
        .ok()
        .and_then(|u| u.join("/").ok())
    else {
        return;
    };
    let icon = find_icon(&state.fetcher, &site).await;
    let (ty, data) = match icon {
        Some((ty, data)) => (Some(ty), Some(data)),
        None => (None, None),
    };
    let res = sqlx::query(
        "INSERT INTO favicons (feed_id, content_type, data, fetched_at) VALUES (?, ?, ?, ?)
         ON CONFLICT (feed_id) DO UPDATE SET content_type = excluded.content_type,
             data = excluded.data, fetched_at = excluded.fetched_at",
    )
    .bind(feed_id)
    .bind(ty)
    .bind(data)
    .bind(t)
    .execute(&state.db)
    .await;
    if let Err(e) = res {
        tracing::warn!(feed_id, error = %e, "storing favicon failed");
    }
}

pub async fn purge_old_entries(state: &AppState) -> AppResult<u64> {
    let cutoff = now() - (state.cfg.retention_days as i64) * 24 * 60 * 60;
    let res = sqlx::query(
        "DELETE FROM entries WHERE is_read = 1 AND is_starred = 0 AND published_at < ?
           AND last_seen_at < (SELECT COALESCE(last_success_at, 0) FROM feeds WHERE feeds.id = entries.feed_id)",
    )
    .bind(cutoff)
    .execute(&state.db)
    .await?;
    sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
        .bind(now())
        .execute(&state.db)
        .await?;
    Ok(res.rows_affected())
}

pub async fn import_opml(state: &AppState, user_id: i64, items: Vec<OpmlFeed>) -> AppResult<(usize, usize)> {
    let (mut added, mut skipped) = (0, 0);
    let t = now();
    for item in items {
        let Ok(url) = parse_http_url(&item.url) else {
            skipped += 1;
            continue;
        };
        let folder_id = match item.folder.as_deref() {
            Some(name) => Some(folder_by_name(&state.db, user_id, name).await?),
            None => None,
        };
        let title = item.title.unwrap_or_else(|| url.to_string());
        let res = sqlx::query(
            "INSERT INTO feeds (user_id, folder_id, url, title, next_fetch_at, created_at)
             VALUES (?, ?, ?, ?, 0, ?) ON CONFLICT (user_id, url) DO NOTHING",
        )
        .bind(user_id)
        .bind(folder_id)
        .bind(url.as_str())
        .bind(title)
        .bind(t)
        .execute(&state.db)
        .await?;
        if res.rows_affected() == 1 {
            added += 1;
        } else {
            skipped += 1;
        }
    }
    state.wake_scheduler.notify_one();
    Ok((added, skipped))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_alternate_links() {
        let html = br#"<html><head>
<link rel="alternate" type="application/rss+xml" title="Posts" href="/feed.xml">
<link rel="alternate" type="application/atom+xml" href="https://other.example/atom">
<link rel="stylesheet" href="/s.css">
<link rel="alternate" type="text/html" href="/fr">
</head></html>"#;
        let base = Url::parse("https://example.com/blog/").unwrap();
        let found = alternate_links(html, &base);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].url, "https://example.com/feed.xml");
        assert_eq!(found[0].title.as_deref(), Some("Posts"));
        assert_eq!(found[1].url, "https://other.example/atom");
    }

    #[test]
    fn finds_icons_in_order() {
        let html = br#"<link rel="apple-touch-icon" href="/apple.png"><link rel="Shortcut Icon" href="/fav.png">"#;
        let base = Url::parse("https://example.com/").unwrap();
        let icons = icon_links(html, &base);
        assert_eq!(icons[0].as_str(), "https://example.com/fav.png");
        assert_eq!(icons[1].as_str(), "https://example.com/apple.png");
    }

    #[test]
    fn sniffs_images() {
        assert_eq!(sniff_image(b"\x89PNG\r\n"), Some("image/png"));
        assert_eq!(sniff_image(b"<svg onload=alert(1)>"), None);
        assert_eq!(sniff_image(b"RIFF1234WEBPVP8"), Some("image/webp"));
    }

    #[test]
    fn normalizes_input() {
        assert_eq!(normalize_input("example.com").unwrap().as_str(), "https://example.com/");
        assert!(normalize_input("ftp://example.com").is_err());
    }
}
