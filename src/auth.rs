use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method, header};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::db::now;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub const SESSION_COOKIE: &str = "yesrss_session";
pub const CSRF_HEADER: &str = "x-yesrss";
pub const WEB_SESSION_TTL: i64 = 60 * 60 * 24 * 90;
pub const API_SESSION_TTL: i64 = 60 * 60 * 24 * 365 * 5;
pub const MIN_PASSWORD_LEN: usize = 8;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionKind {
    Web,
    Api,
}

impl SessionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Api => "api",
        }
    }
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| anyhow::anyhow!("password hashing failed: {e}"))
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    Argon2::default()
        .verify_password(password.as_bytes(), hash)
        .is_ok()
}

fn dummy_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| hash_password("timing-equalizer").expect("argon2 works"))
}

pub fn validate_new_credentials(username: &str, password: &str) -> AppResult<()> {
    let name_ok = !username.is_empty()
        && username.len() <= 64
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '@'));
    if !name_ok {
        return Err(AppError::bad(
            "username must be 1-64 characters of letters, digits, _ - . @",
        ));
    }
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(AppError::bad(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        )));
    }
    Ok(())
}

pub fn new_token() -> String {
    let mut buf = [0u8; 32];
    rand::fill(&mut buf);
    hex::encode(buf)
}

pub fn token_hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

pub async fn create_user(db: &SqlitePool, username: &str, password: &str) -> AppResult<User> {
    validate_new_credentials(username, password)?;
    let hash = hash_blocking(password).await?;
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, password_hash, created_at) VALUES (?, ?, ?)
         RETURNING id, username",
    )
    .bind(username)
    .bind(hash)
    .bind(now())
    .fetch_one(db)
    .await?;
    Ok(user)
}

pub async fn set_password(db: &SqlitePool, user_id: i64, password: &str) -> AppResult<()> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(AppError::bad(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        )));
    }
    let hash = hash_blocking(password).await?;
    let mut tx = db.begin().await?;
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(hash)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM sessions WHERE user_id = ?")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

async fn hash_blocking(password: &str) -> AppResult<String> {
    let password = password.to_owned();
    Ok(tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(anyhow::Error::from)??)
}

pub async fn check_credentials(
    db: &SqlitePool,
    username: &str,
    password: &str,
) -> AppResult<Option<User>> {
    let row: Option<(i64, String, String)> =
        sqlx::query_as("SELECT id, username, password_hash FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(db)
            .await?;
    let password = password.to_owned();
    let (user, hash) = match row {
        Some((id, username, hash)) => (Some(User { id, username }), hash),
        None => (None, dummy_hash().to_owned()),
    };
    let ok = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(anyhow::Error::from)?;
    Ok(user.filter(|_| ok))
}

pub async fn create_session(db: &SqlitePool, user_id: i64, kind: SessionKind) -> AppResult<String> {
    let token = new_token();
    let ttl = match kind {
        SessionKind::Web => WEB_SESSION_TTL,
        SessionKind::Api => API_SESSION_TTL,
    };
    let t = now();
    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, kind, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(token_hash(&token))
    .bind(user_id)
    .bind(kind.as_str())
    .bind(t)
    .bind(t + ttl)
    .execute(db)
    .await?;
    Ok(token)
}

pub async fn delete_session(db: &SqlitePool, token: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
        .bind(token_hash(token))
        .execute(db)
        .await?;
    Ok(())
}

pub async fn user_for_token(
    db: &SqlitePool,
    token: &str,
    kind: SessionKind,
) -> AppResult<Option<User>> {
    Ok(sqlx::query_as::<_, User>(
        "SELECT u.id, u.username FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = ? AND s.kind = ? AND s.expires_at > ?",
    )
    .bind(token_hash(token))
    .bind(kind.as_str())
    .bind(now())
    .fetch_optional(db)
    .await?)
}

pub fn cookie_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| v)
}

pub fn session_cookie(token: &str, secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={WEB_SESSION_TTL}{secure}"
    )
}

pub fn clear_session_cookie(secure: bool) -> String {
    let secure = if secure { "; Secure" } else { "" };
    format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0{secure}")
}

/// A user authenticated by web session cookie; state-changing requests also need `X-Yesrss`.
impl FromRequestParts<AppState> for User {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> AppResult<Self> {
        let safe = matches!(parts.method, Method::GET | Method::HEAD | Method::OPTIONS);
        if !safe && !parts.headers.contains_key(CSRF_HEADER) {
            return Err(AppError::Forbidden);
        }
        let token = cookie_value(&parts.headers, SESSION_COOKIE).ok_or(AppError::Unauthorized)?;
        user_for_token(&state.db, token, SessionKind::Web)
            .await?
            .ok_or(AppError::Unauthorized)
    }
}

const WINDOW: Duration = Duration::from_secs(15 * 60);
const MAX_FAILURES: u32 = 10;

#[derive(Default)]
pub struct RateLimiter {
    failures: Mutex<HashMap<String, (u32, Instant)>>,
}

impl RateLimiter {
    pub fn check(&self, key: &str) -> AppResult<()> {
        let map = self.failures.lock().unwrap();
        match map.get(key) {
            Some((n, start)) if *n >= MAX_FAILURES && start.elapsed() < WINDOW => {
                Err(AppError::TooManyRequests)
            }
            _ => Ok(()),
        }
    }

    pub fn fail(&self, key: &str) {
        let mut map = self.failures.lock().unwrap();
        map.retain(|_, (_, start)| start.elapsed() < WINDOW);
        let entry = map.entry(key.to_owned()).or_insert((0, Instant::now()));
        entry.0 += 1;
    }

    pub fn succeed(&self, key: &str) {
        self.failures.lock().unwrap().remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_roundtrip() {
        let h = hash_password("correct horse").unwrap();
        assert!(verify_password("correct horse", &h));
        assert!(!verify_password("battery staple", &h));
    }

    #[test]
    fn limiter_blocks_after_failures() {
        let l = RateLimiter::default();
        for _ in 0..MAX_FAILURES {
            assert!(l.check("bob").is_ok());
            l.fail("bob");
        }
        assert!(l.check("bob").is_err());
        assert!(l.check("alice").is_ok());
        l.succeed("bob");
        assert!(l.check("bob").is_ok());
    }

    #[test]
    fn parses_cookies() {
        let mut h = HeaderMap::new();
        h.insert(header::COOKIE, "a=1; yesrss_session=abc; b=2".parse().unwrap());
        assert_eq!(cookie_value(&h, SESSION_COOKIE), Some("abc"));
        assert_eq!(cookie_value(&h, "zzz"), None);
    }
}
