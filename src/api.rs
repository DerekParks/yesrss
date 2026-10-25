use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::auth::{
    self, SESSION_COOKIE, SessionKind, User, check_credentials, clear_session_cookie, cookie_value,
    create_session, create_user, session_cookie,
};
use crate::db::now;
use crate::entries::{self, Cursor, Filter, MarkScope, ReadFilter};
use crate::error::{AppError, AppResult};
use crate::feeds;
use crate::opml;
use crate::state::AppState;

const DEFAULT_PAGE: i64 = 40;
const MAX_PAGE: i64 = 200;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/setup", get(setup_status).post(setup))
        .route("/session", get(session))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/password", post(change_password))
        .route("/tree", get(tree))
        .route("/entries", get(list_entries))
        .route("/entries/mark-read", post(mark_read))
        .route("/entries/{id}", patch(update_entry))
        .route("/discover", post(discover))
        .route("/feeds", post(add_feed))
        .route("/feeds/{id}", patch(update_feed).delete(delete_feed))
        .route("/feeds/{id}/refresh", post(refresh_one))
        .route("/feeds/{id}/icon", get(feed_icon))
        .route("/refresh", post(refresh_all))
        .route("/folders", post(add_folder))
        .route("/folders/{id}", patch(rename_folder).delete(delete_folder))
        .route("/opml", get(export_opml).post(import_opml))
}

#[derive(Deserialize)]
struct Credentials {
    username: String,
    password: String,
}

async fn user_count(state: &AppState) -> AppResult<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(&state.db).await?)
}

async fn setup_status(State(state): State<AppState>) -> AppResult<Json<Value>> {
    Ok(Json(json!({ "needed": user_count(&state).await? == 0 })))
}

async fn start_session(state: &AppState, user: User) -> AppResult<Response> {
    let token = create_session(&state.db, user.id, SessionKind::Web).await?;
    Ok((
        [(header::SET_COOKIE, session_cookie(&token, state.cfg.secure_cookies))],
        Json(json!({ "user": user })),
    )
        .into_response())
}

async fn setup(State(state): State<AppState>, headers: HeaderMap, Json(c): Json<Credentials>) -> AppResult<Response> {
    if !headers.contains_key(auth::CSRF_HEADER) {
        return Err(AppError::Forbidden);
    }
    if user_count(&state).await? != 0 {
        return Err(AppError::Conflict("setup already completed".into()));
    }
    let user = create_user(&state.db, c.username.trim(), &c.password).await?;
    tracing::info!(username = %user.username, "created first user via web setup");
    start_session(&state, user).await
}

async fn session(user: User) -> Json<Value> {
    Json(json!({ "user": user }))
}

async fn login(State(state): State<AppState>, headers: HeaderMap, Json(c): Json<Credentials>) -> AppResult<Response> {
    if !headers.contains_key(auth::CSRF_HEADER) {
        return Err(AppError::Forbidden);
    }
    let key = c.username.trim().to_lowercase();
    state.limiter.check(&key)?;
    match check_credentials(&state.db, c.username.trim(), &c.password).await? {
        Some(user) => {
            state.limiter.succeed(&key);
            start_session(&state, user).await
        }
        None => {
            state.limiter.fail(&key);
            Err(AppError::BadRequest("invalid username or password".into()))
        }
    }
}

async fn logout(State(state): State<AppState>, _user: User, headers: HeaderMap) -> AppResult<Response> {
    if let Some(token) = cookie_value(&headers, SESSION_COOKIE) {
        auth::delete_session(&state.db, token).await?;
    }
    Ok((
        [(header::SET_COOKIE, clear_session_cookie(state.cfg.secure_cookies))],
        StatusCode::NO_CONTENT,
    )
        .into_response())
}

#[derive(Deserialize)]
struct PasswordChange {
    current: String,
    new: String,
}

async fn change_password(
    State(state): State<AppState>,
    user: User,
    Json(p): Json<PasswordChange>,
) -> AppResult<Response> {
    let key = user.username.to_lowercase();
    state.limiter.check(&key)?;
    if check_credentials(&state.db, &user.username, &p.current).await?.is_none() {
        state.limiter.fail(&key);
        return Err(AppError::bad("current password is incorrect"));
    }
    auth::set_password(&state.db, user.id, &p.new).await?;
    start_session(&state, user).await
}

#[derive(Serialize, sqlx::FromRow)]
struct FeedInfo {
    id: i64,
    folder_id: Option<i64>,
    title: String,
    url: String,
    site_url: Option<String>,
    unread: i64,
    error_count: i64,
    last_error: Option<String>,
    last_fetched_at: Option<i64>,
    last_fetched_ok: bool,
    has_icon: bool,
}

#[derive(Serialize, sqlx::FromRow)]
struct FolderInfo {
    id: i64,
    name: String,
}

async fn tree(State(state): State<AppState>, user: User) -> AppResult<Json<Value>> {
    let feeds: Vec<FeedInfo> = sqlx::query_as(
        "SELECT f.id, f.folder_id, COALESCE(f.user_title, f.title) AS title, f.url, f.site_url,
                (SELECT COUNT(*) FROM entries e WHERE e.feed_id = f.id AND e.is_read = 0) AS unread,
                f.error_count, f.last_error, f.last_fetched_at, f.last_success_at IS NOT NULL AS last_fetched_ok,
                EXISTS (SELECT 1 FROM favicons i WHERE i.feed_id = f.id AND i.data IS NOT NULL) AS has_icon
         FROM feeds f WHERE f.user_id = ? ORDER BY title COLLATE NOCASE",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    let folders: Vec<FolderInfo> =
        sqlx::query_as("SELECT id, name FROM folders WHERE user_id = ? ORDER BY name COLLATE NOCASE")
            .bind(user.id)
            .fetch_all(&state.db)
            .await?;
    let starred: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM entries WHERE user_id = ? AND is_starred = 1")
            .bind(user.id)
            .fetch_one(&state.db)
            .await?;
    let unread: i64 = feeds.iter().map(|f| f.unread).sum();
    Ok(Json(json!({ "folders": folders, "feeds": feeds, "unread": unread, "starred": starred })))
}

#[derive(Deserialize)]
struct EntryQuery {
    feed: Option<i64>,
    folder: Option<i64>,
    #[serde(default)]
    unread: bool,
    #[serde(default)]
    starred: bool,
    q: Option<String>,
    cursor: Option<String>,
    limit: Option<i64>,
    #[serde(default)]
    oldest: bool,
}

async fn list_entries(
    State(state): State<AppState>,
    user: User,
    Query(q): Query<EntryQuery>,
) -> AppResult<Json<Value>> {
    let as_of = now();
    let filter = Filter {
        user_id: user.id,
        feed_id: q.feed,
        folder_id: q.folder,
        read: if q.unread { ReadFilter::Unread } else { ReadFilter::Any },
        starred: q.starred,
        search: q.q.filter(|s| !s.trim().is_empty()),
        oldest_first: q.oldest,
        cursor: q.cursor.as_deref().and_then(Cursor::decode),
        limit: q.limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE),
        ..Default::default()
    };
    let (rows, next) = entries::list(&state.db, &filter).await?;
    Ok(Json(json!({
        "entries": rows,
        "next": next.map(|c| c.encode()),
        "as_of": as_of,
    })))
}

#[derive(Deserialize)]
struct EntryPatch {
    is_read: Option<bool>,
    is_starred: Option<bool>,
}

async fn update_entry(
    State(state): State<AppState>,
    user: User,
    Path(id): Path<i64>,
    Json(p): Json<EntryPatch>,
) -> AppResult<StatusCode> {
    if let Some(read) = p.is_read {
        entries::set_read(&state.db, user.id, &[id], read).await?;
    }
    if let Some(starred) = p.is_starred {
        entries::set_starred(&state.db, user.id, &[id], starred).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct MarkRead {
    feed: Option<i64>,
    folder: Option<i64>,
    #[serde(default)]
    starred: bool,
    as_of: Option<i64>,
}

async fn mark_read(State(state): State<AppState>, user: User, Json(m): Json<MarkRead>) -> AppResult<Json<Value>> {
    let n = entries::mark_all_read(
        &state.db,
        user.id,
        MarkScope {
            feed_id: m.feed,
            folder_id: m.folder,
            starred: m.starred,
            created_before: Some(m.as_of.unwrap_or_else(now)),
            published_before: None,
        },
    )
    .await?;
    Ok(Json(json!({ "marked": n })))
}

#[derive(Deserialize)]
struct UrlBody {
    url: String,
}

async fn discover(State(state): State<AppState>, _user: User, Json(b): Json<UrlBody>) -> AppResult<Json<Value>> {
    let candidates = feeds::discover(&state, &b.url).await?;
    Ok(Json(json!({ "candidates": candidates })))
}

#[derive(Deserialize)]
struct NewFeed {
    url: String,
    folder_id: Option<i64>,
    folder_name: Option<String>,
    title: Option<String>,
}

async fn add_feed(State(state): State<AppState>, user: User, Json(f): Json<NewFeed>) -> AppResult<Json<Value>> {
    let folder_id = match f.folder_name.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(name) => Some(feeds::folder_by_name(&state.db, user.id, name).await?),
        None => f.folder_id,
    };
    let id = feeds::subscribe(&state, user.id, &f.url, folder_id, f.title.as_deref()).await?;
    Ok(Json(json!({ "id": id })))
}

async fn owned_feed(state: &AppState, user: &User, id: i64) -> AppResult<()> {
    sqlx::query_scalar::<_, i64>("SELECT id FROM feeds WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(())
}

#[derive(Deserialize)]
struct FeedPatch {
    title: Option<String>,
    #[serde(default, with = "double_option")]
    folder_id: Option<Option<i64>>,
    url: Option<String>,
}

mod double_option {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<i64>>, D::Error> {
        Option::<i64>::deserialize(d).map(Some)
    }
}

async fn update_feed(
    State(state): State<AppState>,
    user: User,
    Path(id): Path<i64>,
    Json(p): Json<FeedPatch>,
) -> AppResult<StatusCode> {
    owned_feed(&state, &user, id).await?;
    if let Some(title) = p.title {
        let title = title.trim();
        sqlx::query("UPDATE feeds SET user_title = ? WHERE id = ?")
            .bind((!title.is_empty()).then_some(title))
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(folder_id) = p.folder_id {
        feeds::check_folder(&state.db, user.id, folder_id).await?;
        sqlx::query("UPDATE feeds SET folder_id = ? WHERE id = ?")
            .bind(folder_id)
            .bind(id)
            .execute(&state.db)
            .await?;
    }
    if let Some(url) = p.url {
        let url = crate::fetch::parse_http_url(&url).map_err(|e| AppError::bad(e.to_string()))?;
        sqlx::query(
            "UPDATE feeds SET url = ?, etag = NULL, last_modified = NULL, next_fetch_at = 0 WHERE id = ?",
        )
        .bind(url.as_str())
        .bind(id)
        .execute(&state.db)
        .await?;
        state.wake_scheduler.notify_one();
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_feed(State(state): State<AppState>, user: User, Path(id): Path<i64>) -> AppResult<StatusCode> {
    owned_feed(&state, &user, id).await?;
    sqlx::query("DELETE FROM feeds WHERE id = ?").bind(id).execute(&state.db).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn refresh_one(State(state): State<AppState>, user: User, Path(id): Path<i64>) -> AppResult<Json<Value>> {
    owned_feed(&state, &user, id).await?;
    let added = feeds::refresh_feed(&state, id).await?;
    Ok(Json(json!({ "new": added })))
}

async fn refresh_all(State(state): State<AppState>, user: User) -> AppResult<StatusCode> {
    sqlx::query("UPDATE feeds SET next_fetch_at = 0 WHERE user_id = ?")
        .bind(user.id)
        .execute(&state.db)
        .await?;
    state.wake_scheduler.notify_one();
    Ok(StatusCode::ACCEPTED)
}

async fn feed_icon(State(state): State<AppState>, user: User, Path(id): Path<i64>) -> AppResult<Response> {
    let row: Option<(Option<String>, Option<Vec<u8>>)> = sqlx::query_as(
        "SELECT i.content_type, i.data FROM favicons i JOIN feeds f ON f.id = i.feed_id
         WHERE i.feed_id = ? AND f.user_id = ?",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?;
    let Some((Some(ty), Some(data))) = row else {
        return Err(AppError::NotFound);
    };
    Ok((
        [
            (header::CONTENT_TYPE, ty),
            (header::CACHE_CONTROL, "private, max-age=86400".into()),
            (header::CONTENT_SECURITY_POLICY, "default-src 'none'; sandbox".into()),
        ],
        data,
    )
        .into_response())
}

#[derive(Deserialize)]
struct FolderBody {
    name: String,
}

async fn add_folder(State(state): State<AppState>, user: User, Json(b): Json<FolderBody>) -> AppResult<Json<Value>> {
    let id = feeds::folder_by_name(&state.db, user.id, &b.name).await?;
    Ok(Json(json!({ "id": id })))
}

async fn rename_folder(
    State(state): State<AppState>,
    user: User,
    Path(id): Path<i64>,
    Json(b): Json<FolderBody>,
) -> AppResult<StatusCode> {
    let name = b.name.trim();
    if name.is_empty() || name.len() > 200 {
        return Err(AppError::bad("folder name must be 1-200 characters"));
    }
    let res = sqlx::query("UPDATE folders SET name = ? WHERE id = ? AND user_id = ?")
        .bind(name)
        .bind(id)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_folder(State(state): State<AppState>, user: User, Path(id): Path<i64>) -> AppResult<StatusCode> {
    let res = sqlx::query("DELETE FROM folders WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn export_opml(State(state): State<AppState>, user: User) -> AppResult<Response> {
    let xml = opml::export(&state.db, user.id, &user.username).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/x-opml; charset=utf-8"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"yesrss-subscriptions.opml\""),
        ],
        xml,
    )
        .into_response())
}

async fn import_opml(State(state): State<AppState>, user: User, body: Bytes) -> AppResult<Json<Value>> {
    let xml = String::from_utf8_lossy(&body);
    let items = opml::parse(&xml)?;
    let (added, skipped) = feeds::import_opml(&state, user.id, items).await?;
    Ok(Json(json!({ "added": added, "skipped": skipped })))
}
