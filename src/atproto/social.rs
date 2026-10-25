use std::collections::{HashMap, HashSet};

use futures_util::{StreamExt, stream};
use reqwest::Method;
use serde::Serialize;
use serde_json::{Value, json};
use url::Url;

use super::identity::{self, is_did};
use crate::error::{AppError, AppResult};
use crate::fetch::FetchOutcome;
use crate::state::AppState;

pub const RECOMMEND: &str = "app.skyreader.social.recommend";
pub const STANDARD_RECOMMEND: &str = "site.standard.graph.recommend";
pub const STANDARD_DOCUMENT: &str = "site.standard.document";
pub const STANDARD_PUBLICATION: &str = "site.standard.publication";
const PAGE_MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_FOLLOW_PAGES: usize = 50;
const MAX_RELAY_PAGES: usize = 200;
const MAX_PUBLISHERS: usize = 150;
const RESOLVE_CONCURRENCY: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtUri {
    pub did: String,
    pub collection: String,
    pub rkey: String,
}

impl AtUri {
    pub fn parse(s: &str) -> Option<Self> {
        let rest = s.strip_prefix("at://")?;
        let mut parts = rest.split('/');
        let (did, collection, rkey) = (parts.next()?, parts.next()?, parts.next()?);
        let rkey_ok = !rkey.is_empty()
            && rkey.len() <= 512
            && rkey.chars().all(|c| c.is_ascii_alphanumeric() || "._:~-".contains(c));
        let nsid_ok = collection.split('.').count() >= 3
            && collection.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
        (parts.next().is_none() && is_did(did) && nsid_ok && rkey_ok).then(|| Self {
            did: did.into(),
            collection: collection.into(),
            rkey: rkey.into(),
        })
    }
}

fn timestamp() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Finds `<link rel="site.standard.document" href="at://…">` in an article page.
pub fn standard_document_link(html: &[u8]) -> Option<AtUri> {
    let doc = scraper::Html::parse_document(&String::from_utf8_lossy(html));
    let sel = scraper::Selector::parse("link[rel][href]").expect("valid selector");
    doc.select(&sel)
        .filter(|el| {
            el.value()
                .attr("rel")
                .is_some_and(|r| r.split_whitespace().any(|t| t.eq_ignore_ascii_case(STANDARD_DOCUMENT)))
        })
        .filter_map(|el| AtUri::parse(el.value().attr("href")?.trim()))
        .find(|u| u.collection == STANDARD_DOCUMENT)
}

async fn record_exists(state: &AppState, uri: &AtUri) -> bool {
    let Some(at) = state.atproto.as_ref() else { return false };
    let Ok((_, pds)) = identity::resolve_did(&state.fetcher, &at.net, &uri.did).await else { return false };
    let mut url = pds.join("/xrpc/com.atproto.repo.getRecord").unwrap();
    url.query_pairs_mut()
        .append_pair("repo", &uri.did)
        .append_pair("collection", &uri.collection)
        .append_pair("rkey", &uri.rkey);
    identity::get_json(&state.fetcher, &url).await.is_ok()
}

async fn find_standard_document(state: &AppState, article: &str) -> Option<AtUri> {
    let page = match state.fetcher.get(article, None, None, PAGE_MAX_BYTES).await {
        Ok(FetchOutcome::Fetched(p)) => p,
        _ => return None,
    };
    let uri = standard_document_link(&page.body)?;
    record_exists(state, &uri).await.then_some(uri)
}

fn atproto(state: &AppState) -> AppResult<&super::Atproto> {
    state
        .atproto
        .as_ref()
        .ok_or_else(|| AppError::bad("atproto is not configured on this server (set YESRSS_PUBLIC_URL)"))
}

async fn create_record(state: &AppState, user_id: i64, collection: &str, record: Value) -> AppResult<String> {
    let at = atproto(state)?;
    let did = at
        .account(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::bad("connect a Bluesky account first (Settings → Bluesky)"))?
        .did;
    let body = json!({ "repo": did, "collection": collection, "record": record });
    let (_, v) = at
        .pds_call(&state.db, &state.fetcher, user_id, Method::POST, "com.atproto.repo.createRecord", Some(&body))
        .await?;
    v["uri"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| AppError::bad("PDS did not return a record URI"))
}

async fn delete_record(state: &AppState, user_id: i64, uri: &str) -> AppResult<()> {
    let at = atproto(state)?;
    let Some(parsed) = AtUri::parse(uri) else { return Ok(()) };
    let body = json!({ "repo": parsed.did, "collection": parsed.collection, "rkey": parsed.rkey });
    at.pds_call(&state.db, &state.fetcher, user_id, Method::POST, "com.atproto.repo.deleteRecord", Some(&body))
        .await?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct LikeState {
    pub like_uri: Option<String>,
    pub document_uri: Option<String>,
}

pub async fn like(state: &AppState, user_id: i64, entry_id: i64) -> AppResult<LikeState> {
    let (url, existing): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT url, like_uri FROM entries WHERE id = ? AND user_id = ?")
            .bind(entry_id)
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound)?;
    if existing.is_some() {
        return Ok(LikeState { like_uri: existing, document_uri: None });
    }
    let url = url.ok_or_else(|| AppError::bad("this item has no link to like"))?;
    let document = find_standard_document(state, &url).await;
    let created = timestamp();
    let doc_str = document.as_ref().map(|d| format!("at://{}/{}/{}", d.did, d.collection, d.rkey));
    let mut record = json!({ "$type": RECOMMEND, "subject": url, "createdAt": created });
    if let Some(d) = &doc_str {
        record["document"] = json!(d);
    }
    let like_uri = create_record(state, user_id, RECOMMEND, record).await?;
    let doc_like = match &doc_str {
        Some(d) => {
            let rec = json!({ "$type": STANDARD_RECOMMEND, "document": d, "createdAt": created });
            match create_record(state, user_id, STANDARD_RECOMMEND, rec).await {
                Ok(uri) => Some(uri),
                Err(e) => {
                    tracing::warn!(error = %e, "standard.site recommend failed");
                    None
                }
            }
        }
        None => None,
    };
    sqlx::query("UPDATE entries SET like_uri = ?, like_doc_uri = ? WHERE id = ?")
        .bind(&like_uri)
        .bind(&doc_like)
        .bind(entry_id)
        .execute(&state.db)
        .await?;
    Ok(LikeState { like_uri: Some(like_uri), document_uri: doc_str })
}

pub async fn unlike(state: &AppState, user_id: i64, entry_id: i64) -> AppResult<()> {
    let (like, doc): (Option<String>, Option<String>) =
        sqlx::query_as("SELECT like_uri, like_doc_uri FROM entries WHERE id = ? AND user_id = ?")
            .bind(entry_id)
            .bind(user_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound)?;
    for uri in [like, doc].into_iter().flatten() {
        delete_record(state, user_id, &uri).await?;
    }
    sqlx::query("UPDATE entries SET like_uri = NULL, like_doc_uri = NULL WHERE id = ?")
        .bind(entry_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct Publication {
    pub did: String,
    pub handle: String,
    pub name: String,
    pub url: String,
    pub description: Option<String>,
    pub subscribed: bool,
}

async fn follows(state: &AppState, net: &identity::Network, did: &str) -> AppResult<HashMap<String, String>> {
    let mut out = HashMap::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MAX_FOLLOW_PAGES {
        let mut url = net.appview.join("/xrpc/app.bsky.graph.getFollows").unwrap();
        url.query_pairs_mut().append_pair("actor", did).append_pair("limit", "100");
        if let Some(c) = &cursor {
            url.query_pairs_mut().append_pair("cursor", c);
        }
        let v = identity::get_json(&state.fetcher, &url).await?;
        for f in v["follows"].as_array().into_iter().flatten() {
            if let (Some(d), Some(h)) = (f["did"].as_str(), f["handle"].as_str()) {
                out.insert(d.to_owned(), h.to_owned());
            }
        }
        cursor = v["cursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
    }
    Ok(out)
}

async fn publishers_among(state: &AppState, net: &identity::Network, candidates: &HashMap<String, String>) -> AppResult<Vec<String>> {
    let mut found = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MAX_RELAY_PAGES {
        let mut url = net.relay.join("/xrpc/com.atproto.sync.listReposByCollection").unwrap();
        url.query_pairs_mut()
            .append_pair("collection", STANDARD_PUBLICATION)
            .append_pair("limit", "2000");
        if let Some(c) = &cursor {
            url.query_pairs_mut().append_pair("cursor", c);
        }
        let v = identity::get_json(&state.fetcher, &url).await?;
        found.extend(
            v["repos"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|r| r["did"].as_str())
                .filter(|d| candidates.contains_key(*d))
                .map(str::to_owned),
        );
        cursor = v["cursor"].as_str().filter(|c| !c.is_empty()).map(str::to_owned);
        if cursor.is_none() || found.len() >= MAX_PUBLISHERS {
            break;
        }
    }
    found.truncate(MAX_PUBLISHERS);
    Ok(found)
}

async fn publications_of(state: &AppState, net: &identity::Network, did: &str, handle: &str) -> Vec<Publication> {
    let Ok((_, pds)) = identity::resolve_did(&state.fetcher, net, did).await else { return vec![] };
    let mut url = pds.join("/xrpc/com.atproto.repo.listRecords").unwrap();
    url.query_pairs_mut()
        .append_pair("repo", did)
        .append_pair("collection", STANDARD_PUBLICATION)
        .append_pair("limit", "25");
    let Ok(v) = identity::get_json(&state.fetcher, &url).await else { return vec![] };
    v["records"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            let val = &r["value"];
            let url = Url::parse(val["url"].as_str()?).ok().filter(|u| matches!(u.scheme(), "https" | "http"))?;
            Some(Publication {
                did: did.to_owned(),
                handle: handle.to_owned(),
                name: val["name"].as_str().unwrap_or(url.as_str()).chars().take(200).collect(),
                url: url.into(),
                description: val["description"].as_str().map(|d| d.chars().take(500).collect()),
                subscribed: false,
            })
        })
        .collect()
}

fn site_key(raw: &str) -> Option<String> {
    let u = Url::parse(raw).ok()?;
    Some(format!("{}{}", u.host_str()?.trim_start_matches("www."), u.path().trim_end_matches('/')))
}

/// standard.site publications by accounts the connected user follows on Bluesky.
pub async fn followed_publications(state: &AppState, user_id: i64) -> AppResult<Vec<Publication>> {
    let at = atproto(state)?;
    let acct = at
        .account(&state.db, user_id)
        .await?
        .ok_or_else(|| AppError::bad("connect a Bluesky account first"))?;
    let follows = follows(state, &at.net, &acct.did).await?;
    let publishers = publishers_among(state, &at.net, &follows).await?;
    let mut pubs: Vec<Publication> = stream::iter(publishers)
        .map(|did| {
            let handle = follows.get(&did).cloned().unwrap_or_default();
            let net = at.net.clone();
            async move { publications_of(state, &net, &did, &handle).await }
        })
        .buffer_unordered(RESOLVE_CONCURRENCY)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect();

    let subscribed: HashSet<String> = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT url, site_url FROM feeds WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .flat_map(|(u, s)| [Some(u), s])
    .flatten()
    .filter_map(|u| site_key(&u))
    .collect();
    for p in &mut pubs {
        p.subscribed = site_key(&p.url).is_some_and(|k| subscribed.iter().any(|s| s == &k || s.starts_with(&format!("{k}/"))));
    }
    pubs.sort_by(|a, b| a.handle.cmp(&b.handle).then(a.name.cmp(&b.name)));
    Ok(pubs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_at_uris() {
        let u = AtUri::parse("at://did:plc:abc/site.standard.document/3lwafzkjqm22k").unwrap();
        assert_eq!(u.did, "did:plc:abc");
        assert_eq!(u.collection, STANDARD_DOCUMENT);
        assert_eq!(u.rkey, "3lwafzkjqm22k");
        assert!(AtUri::parse("at://alice.example.com/app.bsky.feed.post/1").is_none());
        assert!(AtUri::parse("at://did:plc:abc/site.standard.document").is_none());
        assert!(AtUri::parse("at://did:plc:abc/site.standard.document/a/b").is_none());
        assert!(AtUri::parse("https://did:plc:abc/x.y.z/1").is_none());
    }

    #[test]
    fn finds_standard_document_link() {
        let html = br#"<html><head>
<link rel="alternate" href="at://did:plc:abc/site.standard.document/zzz">
<link rel="site.standard.document" href="at://did:plc:abc/site.standard.document/3lwa">
</head></html>"#;
        assert_eq!(standard_document_link(html).unwrap().rkey, "3lwa");
        assert!(standard_document_link(br#"<link rel="site.standard.document" href="https://x">"#).is_none());
        assert!(standard_document_link(br#"<link rel="site.standard.document" href="at://did:plc:a/app.bsky.feed.post/1">"#).is_none());
    }

    #[test]
    fn site_keys_normalize() {
        assert_eq!(site_key("https://www.example.com/blog/").as_deref(), Some("example.com/blog"));
        assert_eq!(site_key("https://example.com").as_deref(), Some("example.com"));
    }
}
