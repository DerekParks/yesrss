use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{Value, json};

use super::oauth::{BROWSER_COOKIE, CallbackParams};
use super::social;
use crate::auth::{User, cookie_value};
use crate::error::{AppError, AppResult};
use crate::sanitize::escape_html;
use crate::state::AppState;

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/oauth/client-metadata.json", get(client_metadata))
        .route("/oauth/jwks.json", get(jwks))
        .route("/oauth/callback", get(callback))
}

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/atproto", get(status).delete(disconnect))
        .route("/atproto/connect", post(connect))
        .route("/atproto/publications", get(publications))
        .route("/entries/{id}/like", post(like).delete(unlike))
}

fn configured(state: &AppState) -> AppResult<&super::Atproto> {
    state.atproto.as_ref().ok_or(AppError::NotFound)
}

async fn client_metadata(State(state): State<AppState>) -> AppResult<Response> {
    let at = configured(&state)?;
    Ok(([(header::CACHE_CONTROL, "public, max-age=600")], Json(at.metadata())).into_response())
}

async fn jwks(State(state): State<AppState>) -> AppResult<Response> {
    let at = configured(&state)?;
    Ok(([(header::CACHE_CONTROL, "public, max-age=600")], Json(at.jwks(&state.db).await?)).into_response())
}

fn browser_cookie(value: &str, max_age: i64, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!("{BROWSER_COOKIE}={value}; Path=/oauth/callback; HttpOnly; SameSite=Lax; Max-Age={max_age}{secure}")
}

fn callback_page(title: &str, message: &str, ok: bool) -> Html<String> {
    let refresh = if ok { r#"<meta http-equiv="refresh" content="0;url=/#/connected">"# } else { "" };
    Html(format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">{refresh}<title>{t} · yesrss</title><link rel="stylesheet" href="/app.css"></head><body><div class="auth"><div class="auth-card"><h1>{t}</h1><p>{m}</p><a class="btn primary" href="/#/connected">Back to yesrss</a></div></div></body></html>"#,
        t = escape_html(title),
        m = escape_html(message),
    ))
}

async fn callback(State(state): State<AppState>, headers: HeaderMap, Query(params): Query<CallbackParams>) -> Response {
    let clear = browser_cookie("", 0, state.cfg.secure_cookies);
    let Some(at) = state.atproto.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let browser = cookie_value(&headers, BROWSER_COOKIE);
    let page = match at.callback(&state.db, &state.fetcher, params, browser).await {
        Ok(_) => callback_page("Bluesky connected", "Taking you back to your reader…", true),
        Err(AppError::Forbidden) => callback_page(
            "Sign-in mismatch",
            "This sign-in was started in a different browser. Start again from yesrss Settings.",
            false,
        ),
        Err(e) => {
            if e.status().is_server_error() {
                tracing::error!(error = ?e, "atproto callback failed");
            }
            callback_page("Couldn’t connect Bluesky", &e.public_message(), false)
        }
    };
    ([(header::SET_COOKIE, clear)], page).into_response()
}

async fn status(State(state): State<AppState>, user: User) -> AppResult<Json<Value>> {
    let Some(at) = state.atproto.as_ref() else {
        return Ok(Json(json!({ "enabled": false, "account": null })));
    };
    let account = at
        .account(&state.db, user.id)
        .await?
        .map(|a| json!({ "did": a.did, "handle": a.handle }));
    Ok(Json(json!({ "enabled": true, "client_id": at.client_id(), "account": account })))
}

#[derive(Deserialize)]
struct ConnectBody {
    handle: String,
}

async fn connect(State(state): State<AppState>, user: User, Json(b): Json<ConnectBody>) -> AppResult<Response> {
    let at = state
        .atproto
        .as_ref()
        .ok_or_else(|| AppError::bad("set YESRSS_PUBLIC_URL to enable Bluesky sign-in"))?;
    let (url, browser) = at.start(&state.db, &state.fetcher, user.id, &b.handle).await?;
    Ok((
        [(header::SET_COOKIE, browser_cookie(&browser, 600, state.cfg.secure_cookies))],
        Json(json!({ "redirect": url.as_str() })),
    )
        .into_response())
}

async fn disconnect(State(state): State<AppState>, user: User) -> AppResult<StatusCode> {
    configured(&state)?.disconnect(&state.db, &state.fetcher, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn publications(State(state): State<AppState>, user: User) -> AppResult<Json<Value>> {
    let pubs = social::followed_publications(&state, user.id).await?;
    Ok(Json(json!({ "publications": pubs })))
}

async fn like(State(state): State<AppState>, user: User, Path(id): Path<i64>) -> AppResult<Json<social::LikeState>> {
    Ok(Json(social::like(&state, user.id, id).await?))
}

async fn unlike(State(state): State<AppState>, user: User, Path(id): Path<i64>) -> AppResult<StatusCode> {
    social::unlike(&state, user.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
