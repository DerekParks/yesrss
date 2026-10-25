//! Google Reader API, as spoken by FreshRSS-compatible clients
//! (Reeder, NetNewsWire, ReadYou, FeedMe, FocusReader, Fluent Reader, ...).

use axum::body::Bytes;
use axum::extract::{FromRequest, FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};

use crate::auth::{SessionKind, User, check_credentials, create_session, token_hash, user_for_token};
use crate::db::now;
use crate::entries::{self, Cursor, EntryRow, Filter, MarkScope, ReadFilter};
use crate::error::AppError;
use crate::feeds;
use crate::opml;
use crate::state::AppState;

const READING_LIST: &str = "user/-/state/com.google/reading-list";
const STARRED: &str = "user/-/state/com.google/starred";
const READ: &str = "user/-/state/com.google/read";
const KEPT_UNREAD: &str = "user/-/state/com.google/kept-unread";
const LABEL_PREFIX: &str = "user/-/label/";
const ITEM_PREFIX: &str = "tag:google.com,2005:reader/item/";
const MAX_FORM_BYTES: usize = 4 * 1024 * 1024;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/accounts/ClientLogin", get(client_login).post(client_login))
        .route("/reader/api/0/token", get(token))
        .route("/reader/api/0/user-info", get(user_info))
        .route("/reader/api/0/subscription/list", get(subscription_list))
        .route("/reader/api/0/subscription/edit", post(subscription_edit))
        .route("/reader/api/0/subscription/quickadd", post(quickadd))
        .route("/reader/api/0/subscription/export", get(export))
        .route("/reader/api/0/subscription/import", post(import))
        .route("/reader/api/0/tag/list", get(tag_list))
        .route("/reader/api/0/unread-count", get(unread_count))
        .route("/reader/api/0/stream/contents", get(stream_contents).post(stream_contents))
        .route("/reader/api/0/stream/contents/{*stream}", get(stream_contents).post(stream_contents))
        .route("/reader/api/0/stream/items/ids", get(item_ids).post(item_ids))
        .route("/reader/api/0/stream/items/contents", get(item_contents).post(item_contents))
        .route("/reader/api/0/edit-tag", post(edit_tag))
        .route("/reader/api/0/mark-all-as-read", post(mark_all_as_read))
        .route("/reader/api/0/rename-tag", post(rename_tag))
        .route("/reader/api/0/disable-tag", post(disable_tag))
}

pub struct GError(StatusCode, String);

impl IntoResponse for GError {
    fn into_response(self) -> Response {
        (self.0, [(header::CONTENT_TYPE, "text/plain; charset=utf-8")], self.1).into_response()
    }
}

impl From<AppError> for GError {
    fn from(e: AppError) -> Self {
        let status = e.status();
        if status.is_server_error() {
            tracing::error!(error = ?e, "greader request failed");
        }
        Self(status, e.public_message())
    }
}

type GResult<T> = Result<T, GError>;

fn bad(msg: &str) -> GError {
    GError(StatusCode::BAD_REQUEST, msg.to_owned())
}

pub struct GUser(User);

impl FromRequestParts<AppState> for GUser {
    type Rejection = GError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> GResult<Self> {
        let unauthorized = || GError(StatusCode::UNAUTHORIZED, "Unauthorized".into());
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("GoogleLogin auth="))
            .map(str::trim)
            .ok_or_else(unauthorized)?;
        let token = token.rsplit('/').next().unwrap_or(token);
        user_for_token(&state.db, token, SessionKind::Api)
            .await?
            .map(GUser)
            .ok_or_else(unauthorized)
    }
}

/// Request parameters merged from the query string and a urlencoded body, keeping repeats.
pub struct Params(Vec<(String, String)>);

impl Params {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    fn all(&self, key: &str) -> impl Iterator<Item = &str> {
        self.0.iter().filter(move |(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    fn int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.parse().ok())
    }
}

impl FromRequest<AppState> for Params {
    type Rejection = GError;

    async fn from_request(req: Request, state: &AppState) -> GResult<Self> {
        let mut pairs: Vec<(String, String)> = req
            .uri()
            .query()
            .map(|q| url::form_urlencoded::parse(q.as_bytes()).into_owned().collect())
            .unwrap_or_default();
        let is_form = req
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_none_or(|ct| ct.starts_with("application/x-www-form-urlencoded"));
        if is_form {
            let body = Bytes::from_request(req, state)
                .await
                .map_err(|_| bad("unreadable body"))?;
            if body.len() > MAX_FORM_BYTES {
                return Err(GError(StatusCode::PAYLOAD_TOO_LARGE, "body too large".into()));
            }
            pairs.extend(url::form_urlencoded::parse(&body).into_owned());
        }
        Ok(Self(pairs))
    }
}

async fn client_login(State(state): State<AppState>, p: Params) -> GResult<Response> {
    let (Some(email), Some(passwd)) = (p.get("Email"), p.get("Passwd")) else {
        return Err(GError(StatusCode::UNAUTHORIZED, "Error=BadAuthentication\n".into()));
    };
    let key = email.trim().to_lowercase();
    state.limiter.check(&key)?;
    let Some(user) = check_credentials(&state.db, email.trim(), passwd).await? else {
        state.limiter.fail(&key);
        return Err(GError(StatusCode::UNAUTHORIZED, "Error=BadAuthentication\n".into()));
    };
    state.limiter.succeed(&key);
    let token = create_session(&state.db, user.id, SessionKind::Api).await?;
    let auth = format!("{}/{token}", user.username);
    Ok(([(header::CONTENT_TYPE, "text/plain")], format!("SID={auth}\nLSID=null\nAuth={auth}\n"))
        .into_response())
}

async fn token(GUser(user): GUser) -> String {
    let t = token_hash(&format!("{}:{}", user.id, user.username));
    format!("{}ZZZZZZZZZZZZZZZZZ", &t[..40])
}

async fn user_info(GUser(user): GUser) -> Json<Value> {
    Json(json!({
        "userId": user.id.to_string(),
        "userName": user.username,
        "userProfileId": user.id.to_string(),
        "userEmail": "",
    }))
}

fn label_id(name: &str) -> String {
    format!("{LABEL_PREFIX}{name}")
}

/// Rewrites `user/1234/...` to `user/-/...`.
fn normalize_stream(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("user/")
        && let Some((_, tail)) = rest.split_once('/')
    {
        return format!("user/-/{tail}");
    }
    s.to_owned()
}

fn feed_stream_id(s: &str) -> Option<i64> {
    s.strip_prefix("feed/")?.parse().ok()
}

fn parse_item_id(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    if let Some(hex) = raw.strip_prefix(ITEM_PREFIX) {
        return u64::from_str_radix(hex, 16).ok().map(|v| v as i64);
    }
    raw.parse::<i64>().ok().or_else(|| u64::from_str_radix(raw, 16).ok().map(|v| v as i64))
}

fn long_item_id(id: i64) -> String {
    format!("{ITEM_PREFIX}{:016x}", id as u64)
}

async fn folder_id_by_label(state: &AppState, user_id: i64, name: &str) -> GResult<Option<i64>> {
    Ok(sqlx::query_scalar("SELECT id FROM folders WHERE user_id = ? AND name = ?")
        .bind(user_id)
        .bind(name)
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::from)?)
}

#[derive(sqlx::FromRow)]
struct SubscriptionRow {
    id: i64,
    title: String,
    url: String,
    site_url: Option<String>,
    folder: Option<String>,
}

async fn subscription_list(State(state): State<AppState>, GUser(user): GUser) -> GResult<Json<Value>> {
    let rows: Vec<SubscriptionRow> = sqlx::query_as(
        "SELECT f.id, COALESCE(f.user_title, f.title) AS title, f.url, f.site_url, fo.name AS folder
         FROM feeds f LEFT JOIN folders fo ON fo.id = f.folder_id
         WHERE f.user_id = ? ORDER BY 2 COLLATE NOCASE",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::from)?;
    let subs: Vec<Value> = rows
        .into_iter()
        .map(|SubscriptionRow { id, title, url, site_url: site, folder }| {
            let categories: Vec<Value> = folder
                .map(|name| json!({ "id": label_id(&name), "label": name }))
                .into_iter()
                .collect();
            json!({
                "id": format!("feed/{id}"),
                "title": title,
                "categories": categories,
                "url": url,
                "htmlUrl": site.unwrap_or_default(),
                "iconUrl": "",
            })
        })
        .collect();
    Ok(Json(json!({ "subscriptions": subs })))
}

async fn subscription_edit(State(state): State<AppState>, GUser(user): GUser, p: Params) -> GResult<&'static str> {
    let action = p.get("ac").unwrap_or("edit");
    let add_label = p
        .get("a")
        .map(normalize_stream)
        .and_then(|a| a.strip_prefix(LABEL_PREFIX).map(str::to_owned));
    let streams: Vec<&str> = p.all("s").collect();
    if streams.is_empty() {
        return Err(bad("missing s"));
    }
    for s in streams {
        match action {
            "subscribe" => {
                let url = s.strip_prefix("feed/").unwrap_or(s);
                let folder_id = match &add_label {
                    Some(name) => Some(feeds::folder_by_name(&state.db, user.id, name).await?),
                    None => None,
                };
                feeds::subscribe(&state, user.id, url, folder_id, p.get("t")).await?;
            }
            "unsubscribe" => {
                let id = feed_stream_id(s).ok_or_else(|| bad("bad stream id"))?;
                sqlx::query("DELETE FROM feeds WHERE id = ? AND user_id = ?")
                    .bind(id)
                    .bind(user.id)
                    .execute(&state.db)
                    .await
                    .map_err(AppError::from)?;
            }
            "edit" => {
                let id = feed_stream_id(s).ok_or_else(|| bad("bad stream id"))?;
                if let Some(title) = p.get("t").map(str::trim).filter(|t| !t.is_empty()) {
                    sqlx::query("UPDATE feeds SET user_title = ? WHERE id = ? AND user_id = ?")
                        .bind(title)
                        .bind(id)
                        .bind(user.id)
                        .execute(&state.db)
                        .await
                        .map_err(AppError::from)?;
                }
                let folder = match (&add_label, p.get("r")) {
                    (Some(name), _) => Some(Some(feeds::folder_by_name(&state.db, user.id, name).await?)),
                    (None, Some(_)) => Some(None),
                    (None, None) => None,
                };
                if let Some(folder_id) = folder {
                    sqlx::query("UPDATE feeds SET folder_id = ? WHERE id = ? AND user_id = ?")
                        .bind(folder_id)
                        .bind(id)
                        .bind(user.id)
                        .execute(&state.db)
                        .await
                        .map_err(AppError::from)?;
                }
            }
            _ => return Err(bad("unknown action")),
        }
    }
    Ok("OK")
}

async fn quickadd(State(state): State<AppState>, GUser(user): GUser, p: Params) -> GResult<Json<Value>> {
    let url = p.get("quickadd").ok_or_else(|| bad("missing quickadd"))?;
    let url = url.strip_prefix("feed/").unwrap_or(url);
    let id = feeds::subscribe(&state, user.id, url, None, None).await?;
    let title: String = sqlx::query_scalar("SELECT title FROM feeds WHERE id = ?")
        .bind(id)
        .fetch_one(&state.db)
        .await
        .map_err(AppError::from)?;
    Ok(Json(json!({
        "numResults": 1,
        "query": url,
        "streamId": format!("feed/{id}"),
        "streamName": title,
    })))
}

async fn export(State(state): State<AppState>, GUser(user): GUser) -> GResult<Response> {
    let xml = opml::export(&state.db, user.id, &user.username).await?;
    Ok(([(header::CONTENT_TYPE, "text/x-opml; charset=utf-8")], xml).into_response())
}

async fn import(State(state): State<AppState>, GUser(user): GUser, body: Bytes) -> GResult<&'static str> {
    let items = opml::parse(&String::from_utf8_lossy(&body))?;
    feeds::import_opml(&state, user.id, items).await?;
    Ok("OK")
}

async fn tag_list(State(state): State<AppState>, GUser(user): GUser) -> GResult<Json<Value>> {
    let names: Vec<String> =
        sqlx::query_scalar("SELECT name FROM folders WHERE user_id = ? ORDER BY name COLLATE NOCASE")
            .bind(user.id)
            .fetch_all(&state.db)
            .await
            .map_err(AppError::from)?;
    let mut tags = vec![json!({ "id": STARRED, "type": "tag" })];
    tags.extend(names.into_iter().map(|n| json!({ "id": label_id(&n), "type": "folder" })));
    Ok(Json(json!({ "tags": tags })))
}

async fn unread_count(State(state): State<AppState>, GUser(user): GUser) -> GResult<Json<Value>> {
    let rows: Vec<(i64, Option<String>, i64, Option<i64>)> = sqlx::query_as(
        "SELECT f.id, fo.name, COUNT(e.id), MAX(e.published_at)
         FROM feeds f LEFT JOIN folders fo ON fo.id = f.folder_id
         LEFT JOIN entries e ON e.feed_id = f.id AND e.is_read = 0
         WHERE f.user_id = ? GROUP BY f.id",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::from)?;
    let usec = |t: Option<i64>| (t.unwrap_or(0) * 1_000_000).to_string();
    let mut counts = Vec::new();
    let mut labels: std::collections::BTreeMap<String, (i64, i64)> = Default::default();
    let (mut total, mut newest) = (0, 0);
    for (id, folder, count, latest) in rows {
        counts.push(json!({ "id": format!("feed/{id}"), "count": count, "newestItemTimestampUsec": usec(latest) }));
        if let Some(name) = folder {
            let e = labels.entry(name).or_default();
            e.0 += count;
            e.1 = e.1.max(latest.unwrap_or(0));
        }
        total += count;
        newest = newest.max(latest.unwrap_or(0));
    }
    for (name, (count, latest)) in labels {
        counts.push(json!({ "id": label_id(&name), "count": count, "newestItemTimestampUsec": usec(Some(latest)) }));
    }
    counts.push(json!({ "id": READING_LIST, "count": total, "newestItemTimestampUsec": usec(Some(newest)) }));
    Ok(Json(json!({ "max": total, "unreadcounts": counts })))
}

/// Builds an entry filter from the stream id and the `xt`/`it`/`ot`/`nt`/`r`/`c`/`n` parameters.
async fn stream_filter(
    state: &AppState,
    user: &User,
    stream: &str,
    p: &Params,
    default_n: i64,
    max_n: i64,
) -> GResult<Filter> {
    let stream = normalize_stream(stream);
    let mut f = Filter {
        user_id: user.id,
        limit: p.int("n").unwrap_or(default_n).clamp(1, max_n),
        oldest_first: p.get("r") == Some("o"),
        cursor: p.get("c").and_then(Cursor::decode),
        published_after: p.int("ot"),
        published_before: p.int("nt"),
        ..Default::default()
    };
    if stream == STARRED {
        f.starred = true;
    } else if stream == READ {
        f.read = ReadFilter::Read;
    } else if let Some(name) = stream.strip_prefix(LABEL_PREFIX) {
        f.folder_id = Some(folder_id_by_label(state, user.id, name).await?.unwrap_or(-1));
    } else if let Some(id) = feed_stream_id(&stream) {
        f.feed_id = Some(id);
    } else if stream != READING_LIST && !stream.is_empty() {
        return Err(bad("unknown stream"));
    }
    for xt in p.all("xt").map(normalize_stream) {
        if xt == READ {
            f.read = ReadFilter::Unread;
        } else if xt == STARRED {
            return Err(bad("excluding starred is not supported"));
        }
    }
    for it in p.all("it").map(normalize_stream) {
        if it == STARRED {
            f.starred = true;
        } else if it == READ {
            f.read = ReadFilter::Read;
        }
    }
    Ok(f)
}

fn item_json(e: &EntryRow) -> Value {
    let mut categories = vec![READING_LIST.to_owned()];
    if let Some(name) = &e.folder_name {
        categories.push(label_id(name));
    }
    if e.is_read {
        categories.push(READ.to_owned());
    }
    if e.is_starred {
        categories.push(STARRED.to_owned());
    }
    let link: Vec<Value> = e.url.iter().map(|u| json!({ "href": u, "type": "text/html" })).collect();
    let enclosure: Vec<Value> = e
        .enclosure_url
        .iter()
        .map(|u| json!({ "href": u, "type": e.enclosure_type.clone().unwrap_or_default() }))
        .collect();
    json!({
        "id": long_item_id(e.id),
        "crawlTimeMsec": (e.created_at * 1000).to_string(),
        "timestampUsec": (e.published_at * 1_000_000).to_string(),
        "published": e.published_at,
        "updated": e.published_at,
        "title": e.title,
        "author": e.author.clone().unwrap_or_default(),
        "canonical": link,
        "alternate": link,
        "categories": categories,
        "origin": {
            "streamId": format!("feed/{}", e.feed_id),
            "title": e.feed_title,
            "htmlUrl": e.feed_site_url.clone().unwrap_or_default(),
        },
        "summary": { "direction": "ltr", "content": e.content },
        "enclosure": enclosure,
    })
}

async fn stream_contents(
    State(state): State<AppState>,
    GUser(user): GUser,
    path: Option<axum::extract::Path<String>>,
    p: Params,
) -> GResult<Json<Value>> {
    let stream = path
        .map(|axum::extract::Path(s)| s)
        .or_else(|| p.get("s").map(str::to_owned))
        .unwrap_or_else(|| READING_LIST.to_owned());
    let filter = stream_filter(&state, &user, &stream, &p, 20, 1000).await?;
    let (rows, next) = entries::list(&state.db, &filter).await?;
    let mut body = json!({
        "id": stream,
        "updated": now(),
        "items": rows.iter().map(item_json).collect::<Vec<_>>(),
    });
    if let Some(c) = next {
        body["continuation"] = json!(c.encode());
    }
    Ok(Json(body))
}

async fn item_ids(State(state): State<AppState>, GUser(user): GUser, p: Params) -> GResult<Json<Value>> {
    let stream = p.get("s").unwrap_or(READING_LIST).to_owned();
    let filter = stream_filter(&state, &user, &stream, &p, 1000, 10_000).await?;
    let (rows, next) = entries::list_refs(&state.db, &filter).await?;
    let refs: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.id.to_string(),
                "directStreamIds": [format!("feed/{}", r.feed_id)],
                "timestampUsec": (r.published_at * 1_000_000).to_string(),
            })
        })
        .collect();
    let mut body = json!({ "itemRefs": refs });
    if let Some(c) = next {
        body["continuation"] = json!(c.encode());
    }
    Ok(Json(body))
}

fn item_ids_param(p: &Params) -> GResult<Vec<i64>> {
    p.all("i")
        .map(|raw| parse_item_id(raw).ok_or_else(|| bad("bad item id")))
        .collect()
}

async fn item_contents(State(state): State<AppState>, GUser(user): GUser, p: Params) -> GResult<Json<Value>> {
    let ids = item_ids_param(&p)?;
    let filter = Filter {
        user_id: user.id,
        limit: ids.len().max(1) as i64,
        ids: Some(ids),
        oldest_first: p.get("r") == Some("o"),
        ..Default::default()
    };
    let (rows, _) = entries::list(&state.db, &filter).await?;
    Ok(Json(json!({
        "id": READING_LIST,
        "updated": now(),
        "items": rows.iter().map(item_json).collect::<Vec<_>>(),
    })))
}

async fn edit_tag(State(state): State<AppState>, GUser(user): GUser, p: Params) -> GResult<&'static str> {
    let ids = item_ids_param(&p)?;
    for (tags, adding) in [(p.all("a"), true), (p.all("r"), false)] {
        for tag in tags.map(normalize_stream) {
            match tag.as_str() {
                READ => entries::set_read(&state.db, user.id, &ids, adding).await?,
                KEPT_UNREAD if adding => entries::set_read(&state.db, user.id, &ids, false).await?,
                STARRED => entries::set_starred(&state.db, user.id, &ids, adding).await?,
                _ => 0,
            };
        }
    }
    Ok("OK")
}

async fn mark_all_as_read(State(state): State<AppState>, GUser(user): GUser, p: Params) -> GResult<&'static str> {
    let stream = normalize_stream(p.get("s").unwrap_or(READING_LIST));
    let mut scope = MarkScope {
        published_before: p
            .int("ts")
            .map(|usec| usec / 1_000_000)
            .filter(|&t| t > 0),
        created_before: Some(now()),
        ..Default::default()
    };
    if stream == STARRED {
        scope.starred = true;
    } else if let Some(name) = stream.strip_prefix(LABEL_PREFIX) {
        scope.folder_id = Some(folder_id_by_label(&state, user.id, name).await?.unwrap_or(-1));
    } else if let Some(id) = feed_stream_id(&stream) {
        scope.feed_id = Some(id);
    } else if stream != READING_LIST {
        return Err(bad("unknown stream"));
    }
    entries::mark_all_read(&state.db, user.id, scope).await?;
    Ok("OK")
}

async fn rename_tag(State(state): State<AppState>, GUser(user): GUser, p: Params) -> GResult<&'static str> {
    let from = p.get("s").map(normalize_stream).ok_or_else(|| bad("missing s"))?;
    let to = p.get("dest").map(normalize_stream).ok_or_else(|| bad("missing dest"))?;
    let (Some(from), Some(to)) = (from.strip_prefix(LABEL_PREFIX), to.strip_prefix(LABEL_PREFIX)) else {
        return Err(bad("only labels can be renamed"));
    };
    let to = to.trim();
    if to.is_empty() {
        return Err(bad("empty label"));
    }
    sqlx::query("UPDATE folders SET name = ? WHERE user_id = ? AND name = ?")
        .bind(to)
        .bind(user.id)
        .bind(from)
        .execute(&state.db)
        .await
        .map_err(AppError::from)?;
    Ok("OK")
}

async fn disable_tag(State(state): State<AppState>, GUser(user): GUser, p: Params) -> GResult<&'static str> {
    let s = p.get("s").map(normalize_stream).ok_or_else(|| bad("missing s"))?;
    let name = s.strip_prefix(LABEL_PREFIX).ok_or_else(|| bad("only labels can be deleted"))?;
    sqlx::query("DELETE FROM folders WHERE user_id = ? AND name = ?")
        .bind(user.id)
        .bind(name)
        .execute(&state.db)
        .await
        .map_err(AppError::from)?;
    Ok("OK")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_ids() {
        assert_eq!(parse_item_id("tag:google.com,2005:reader/item/000000000000001f"), Some(31));
        assert_eq!(parse_item_id("31"), Some(31));
        assert_eq!(long_item_id(31), "tag:google.com,2005:reader/item/000000000000001f");
        assert_eq!(parse_item_id(&long_item_id(123456789)), Some(123456789));
    }

    #[test]
    fn streams() {
        assert_eq!(normalize_stream("user/1/state/com.google/read"), READ);
        assert_eq!(normalize_stream("user/-/label/Tech"), "user/-/label/Tech");
        assert_eq!(normalize_stream("feed/12"), "feed/12");
        assert_eq!(feed_stream_id("feed/12"), Some(12));
        assert_eq!(feed_stream_id("feed/https://x"), None);
    }
}
