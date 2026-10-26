use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use reqwest::Method;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tokio::sync::OnceCell;
use url::Url;

use super::identity::{self, Identity, Network};
use super::jwt::{EsKey, b64, pkce_challenge, random_string};
use crate::auth::token_hash;
use crate::db::now;
use crate::error::{AppError, AppResult};
use crate::fetch::{Body, Fetcher};

pub const SCOPE: &str = "atproto transition:generic";
pub const BROWSER_COOKIE: &str = "yesrss_oauth";
const PENDING_TTL: i64 = 600;
const AS_MAX_BYTES: usize = 256 * 1024;
const PDS_MAX_BYTES: usize = 4 * 1024 * 1024;
const REFRESH_MARGIN: i64 = 60;
const CLIENT_KEY_NAME: &str = "oauth_client_key";
const ASSERTION_TYPE: &str = "urn:ietf:params:oauth:client-assertion-type:jwt-bearer";

pub struct Atproto {
    pub public_url: Url,
    pub net: Network,
    client_key: OnceCell<EsKey>,
    locks: Mutex<HashMap<i64, Arc<tokio::sync::Mutex<()>>>>,
}

#[derive(Debug, Clone)]
struct AuthServer {
    issuer: String,
    authorization_endpoint: Url,
    token_endpoint: Url,
    par_endpoint: Url,
    revocation_endpoint: Option<Url>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Account {
    pub user_id: i64,
    pub did: String,
    pub handle: String,
    pub pds_url: String,
    issuer: String,
    token_endpoint: String,
    revocation_endpoint: Option<String>,
    access_token: String,
    refresh_token: Option<String>,
    expires_at: i64,
    dpop_key: String,
    as_nonce: Option<String>,
    pds_nonce: Option<String>,
}

#[derive(sqlx::FromRow)]
struct Pending {
    browser_hash: String,
    user_id: i64,
    did: String,
    handle: String,
    pds_url: String,
    issuer: String,
    token_endpoint: String,
    revocation_endpoint: Option<String>,
    pkce_verifier: String,
    dpop_key: String,
    dpop_nonce: Option<String>,
    created_at: i64,
}

#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    pub state: Option<String>,
    pub iss: Option<String>,
    pub code: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// A DPoP key and the latest nonce its server issued.
struct Dpop<'a> {
    key: &'a EsKey,
    nonce: &'a mut Option<String>,
}

struct Tokens {
    access: String,
    refresh: Option<String>,
    expires_at: i64,
    scope: String,
}

fn htu(url: &Url) -> String {
    let mut u = url.clone();
    u.set_query(None);
    u.set_fragment(None);
    u.into()
}

fn dpop_proof(key: &EsKey, method: &str, url: &Url, nonce: Option<&str>, access_token: Option<&str>) -> String {
    let mut claims = json!({ "jti": random_string(16), "htm": method, "htu": htu(url), "iat": now() });
    if let Some(n) = nonce {
        claims["nonce"] = json!(n);
    }
    if let Some(t) = access_token {
        claims["ath"] = json!(b64(Sha256::digest(t.as_bytes())));
    }
    key.sign_jwt(&json!({ "typ": "dpop+jwt", "alg": "ES256", "jwk": key.public_jwk() }), &claims)
}

fn header_value(s: &str) -> AppResult<HeaderValue> {
    HeaderValue::from_str(s).map_err(|e| AppError::Other(e.into()))
}

fn require_secure(url: &Url, insecure_ok: bool) -> AppResult<()> {
    if url.scheme() == "https" || (insecure_ok && url.scheme() == "http") {
        Ok(())
    } else {
        Err(AppError::bad(format!("{url} must use https")))
    }
}

fn endpoint(meta: &Value, name: &str, insecure_ok: bool) -> AppResult<Url> {
    let url = meta[name]
        .as_str()
        .and_then(|s| Url::parse(s).ok())
        .ok_or_else(|| AppError::bad(format!("authorization server metadata lacks {name}")))?;
    require_secure(&url, insecure_ok)?;
    Ok(url)
}

fn same_issuer(a: &str, b: &str) -> bool {
    a.trim_end_matches('/') == b.trim_end_matches('/')
}

fn validate_tokens(v: &Value, expected_did: &str) -> AppResult<Tokens> {
    if !v["token_type"].as_str().is_some_and(|t| t.eq_ignore_ascii_case("DPoP")) {
        return Err(AppError::bad("authorization server did not issue a DPoP token"));
    }
    if v["sub"].as_str() != Some(expected_did) {
        return Err(AppError::bad("authorization server returned a different account than requested"));
    }
    let scope = v["scope"].as_str().unwrap_or_default().to_owned();
    if !scope.split_whitespace().any(|s| s == "atproto") {
        return Err(AppError::bad("authorization server did not grant the atproto scope"));
    }
    let access = v["access_token"]
        .as_str()
        .filter(|t| !t.is_empty())
        .ok_or_else(|| AppError::bad("authorization server returned no access token"))?
        .to_owned();
    Ok(Tokens {
        access,
        refresh: v["refresh_token"].as_str().map(str::to_owned),
        expires_at: now() + v["expires_in"].as_i64().unwrap_or(300).clamp(30, 86_400),
        scope,
    })
}

impl Atproto {
    pub fn new(mut public_url: Url, net: Network) -> Self {
        if !public_url.path().ends_with('/') {
            let path = format!("{}/", public_url.path());
            public_url.set_path(&path);
        }
        Self { public_url, net, client_key: OnceCell::new(), locks: Mutex::default() }
    }

    pub fn client_id(&self) -> String {
        self.public_url.join("oauth/client-metadata.json").unwrap().into()
    }

    pub fn redirect_uri(&self) -> String {
        self.public_url.join("oauth/callback").unwrap().into()
    }

    pub fn metadata(&self) -> Value {
        json!({
            "client_id": self.client_id(),
            "application_type": "web",
            "client_name": "yesrss",
            "client_uri": self.public_url.as_str(),
            "redirect_uris": [self.redirect_uri()],
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "scope": SCOPE,
            "token_endpoint_auth_method": "private_key_jwt",
            "token_endpoint_auth_signing_alg": "ES256",
            "jwks_uri": self.public_url.join("oauth/jwks.json").unwrap().as_str(),
            "dpop_bound_access_tokens": true,
        })
    }

    async fn client_key(&self, db: &SqlitePool) -> AppResult<&EsKey> {
        self.client_key
            .get_or_try_init(|| async {
                let fresh = EsKey::generate();
                sqlx::query("INSERT OR IGNORE INTO app_secrets (name, value) VALUES (?, ?)")
                    .bind(CLIENT_KEY_NAME)
                    .bind(fresh.to_hex())
                    .execute(db)
                    .await?;
                let stored: String = sqlx::query_scalar("SELECT value FROM app_secrets WHERE name = ?")
                    .bind(CLIENT_KEY_NAME)
                    .fetch_one(db)
                    .await?;
                Ok::<_, AppError>(EsKey::from_hex(&stored)?)
            })
            .await
    }

    pub async fn jwks(&self, db: &SqlitePool) -> AppResult<Value> {
        let key = self.client_key(db).await?;
        let mut jwk = key.public_jwk();
        jwk["kid"] = json!(key.thumbprint());
        jwk["use"] = json!("sig");
        jwk["alg"] = json!("ES256");
        Ok(json!({ "keys": [jwk] }))
    }

    fn user_lock(&self, user_id: i64) -> Arc<tokio::sync::Mutex<()>> {
        self.locks.lock().unwrap().entry(user_id).or_default().clone()
    }

    async fn client_assertion(&self, db: &SqlitePool, issuer: &str) -> AppResult<String> {
        let key = self.client_key(db).await?;
        let t = now();
        Ok(key.sign_jwt(
            &json!({ "alg": "ES256", "typ": "JWT", "kid": key.thumbprint() }),
            &json!({
                "iss": self.client_id(),
                "sub": self.client_id(),
                "aud": issuer,
                "jti": random_string(16),
                "iat": t,
                "exp": t + 60,
            }),
        ))
    }

    async fn auth_server_for(&self, fetcher: &Fetcher, pds: &Url) -> AppResult<AuthServer> {
        let insecure_ok = fetcher.allows_private();
        let resource = identity::get_json(fetcher, &pds.join("/.well-known/oauth-protected-resource").unwrap()).await?;
        let issuer = resource["authorization_servers"][0]
            .as_str()
            .ok_or_else(|| AppError::bad("PDS does not name an authorization server"))?
            .to_owned();
        let issuer_url = Url::parse(&issuer).map_err(|e| AppError::bad(e.to_string()))?;
        require_secure(&issuer_url, insecure_ok)?;
        let meta = identity::get_json(fetcher, &issuer_url.join("/.well-known/oauth-authorization-server").unwrap()).await?;
        if !meta["issuer"].as_str().is_some_and(|i| same_issuer(i, &issuer)) {
            return Err(AppError::bad("authorization server metadata has a mismatched issuer"));
        }
        let supports = |field: &str, value: &str| {
            meta[field].as_array().is_some_and(|a| a.iter().any(|v| v.as_str() == Some(value)))
        };
        if !supports("token_endpoint_auth_methods_supported", "private_key_jwt")
            || !supports("dpop_signing_alg_values_supported", "ES256")
        {
            return Err(AppError::bad("authorization server lacks private_key_jwt or ES256 DPoP support"));
        }
        Ok(AuthServer {
            issuer: meta["issuer"].as_str().unwrap().to_owned(),
            authorization_endpoint: endpoint(&meta, "authorization_endpoint", insecure_ok)?,
            token_endpoint: endpoint(&meta, "token_endpoint", insecure_ok)?,
            par_endpoint: endpoint(&meta, "pushed_authorization_request_endpoint", insecure_ok)?,
            revocation_endpoint: endpoint(&meta, "revocation_endpoint", insecure_ok).ok(),
        })
    }

    async fn post_as(
        &self,
        db: &SqlitePool,
        fetcher: &Fetcher,
        url: &Url,
        issuer: &str,
        form: &[(String, String)],
        dpop: Dpop<'_>,
    ) -> AppResult<Value> {
        let Dpop { key, nonce } = dpop;
        for attempt in 0..2 {
            let mut pairs = form.to_vec();
            pairs.push(("client_id".into(), self.client_id()));
            pairs.push(("client_assertion_type".into(), ASSERTION_TYPE.into()));
            pairs.push(("client_assertion".into(), self.client_assertion(db, issuer).await?));
            let mut headers = HeaderMap::new();
            headers.insert("dpop", header_value(&dpop_proof(key, "POST", url, nonce.as_deref(), None))?);
            let resp = fetcher
                .request(Method::POST, url, headers, Body::Form(&pairs), AS_MAX_BYTES)
                .await
                .map_err(|e| AppError::bad(format!("{url}: {e}")))?;
            if let Some(n) = resp.header("dpop-nonce") {
                *nonce = Some(n.to_owned());
            }
            let body = resp.json().unwrap_or(Value::Null);
            if resp.status.is_success() {
                return Ok(body);
            }
            if attempt == 0 && body["error"] == "use_dpop_nonce" {
                continue;
            }
            return Err(AppError::bad(format!(
                "authorization server: {} {}",
                body["error"].as_str().unwrap_or(resp.status.as_str()),
                body["error_description"].as_str().unwrap_or_default()
            )));
        }
        unreachable!("loop returns on its final attempt")
    }

    /// Begins sign-in; returns the authorization URL and a secret for the browser-binding cookie.
    pub async fn start(&self, db: &SqlitePool, fetcher: &Fetcher, user_id: i64, input: &str) -> AppResult<(Url, String)> {
        let Identity { did, handle, pds } = identity::resolve(fetcher, &self.net, input).await?;
        let asv = self.auth_server_for(fetcher, &pds).await?;
        let verifier = random_string(32);
        let state = random_string(16);
        let browser = random_string(16);
        let key = EsKey::generate();
        let mut nonce = None;
        let form = vec![
            ("response_type".to_owned(), "code".to_owned()),
            ("code_challenge".to_owned(), pkce_challenge(&verifier)),
            ("code_challenge_method".to_owned(), "S256".to_owned()),
            ("state".to_owned(), state.clone()),
            ("redirect_uri".to_owned(), self.redirect_uri()),
            ("scope".to_owned(), SCOPE.to_owned()),
            ("login_hint".to_owned(), handle.clone()),
        ];
        let par = self.post_as(db, fetcher, &asv.par_endpoint, &asv.issuer, &form, Dpop { key: &key, nonce: &mut nonce }).await?;
        let request_uri = par["request_uri"]
            .as_str()
            .ok_or_else(|| AppError::bad("authorization server returned no request_uri"))?;

        sqlx::query("DELETE FROM oauth_pending WHERE created_at < ?")
            .bind(now() - PENDING_TTL)
            .execute(db)
            .await?;
        sqlx::query(
            "INSERT INTO oauth_pending (state, browser_hash, user_id, did, handle, pds_url, issuer, token_endpoint,
                 revocation_endpoint, pkce_verifier, dpop_key, dpop_nonce, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&state)
        .bind(token_hash(&browser))
        .bind(user_id)
        .bind(&did)
        .bind(&handle)
        .bind(pds.as_str())
        .bind(&asv.issuer)
        .bind(asv.token_endpoint.as_str())
        .bind(asv.revocation_endpoint.as_ref().map(Url::as_str))
        .bind(&verifier)
        .bind(key.to_hex())
        .bind(&nonce)
        .bind(now())
        .execute(db)
        .await?;

        let mut url = asv.authorization_endpoint.clone();
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id())
            .append_pair("request_uri", request_uri);
        Ok((url, browser))
    }

    /// Completes sign-in. `browser` is the binding cookie set by `start`.
    pub async fn callback(
        &self,
        db: &SqlitePool,
        fetcher: &Fetcher,
        params: CallbackParams,
        browser: Option<&str>,
    ) -> AppResult<i64> {
        let state = params.state.as_deref().ok_or_else(|| AppError::bad("missing state"))?;
        let pending: Pending = sqlx::query_as("DELETE FROM oauth_pending WHERE state = ? RETURNING *")
            .bind(state)
            .fetch_optional(db)
            .await?
            .ok_or_else(|| AppError::bad("this sign-in link expired or was already used"))?;
        if now() - pending.created_at > PENDING_TTL {
            return Err(AppError::bad("this sign-in link expired"));
        }
        if browser.map(token_hash).as_deref() != Some(pending.browser_hash.as_str()) {
            return Err(AppError::Forbidden);
        }
        if let Some(err) = params.error {
            return Err(AppError::bad(format!(
                "sign-in was not completed: {err} {}",
                params.error_description.unwrap_or_default()
            )));
        }
        if !params.iss.as_deref().is_some_and(|i| same_issuer(i, &pending.issuer)) {
            return Err(AppError::bad("sign-in response came from an unexpected issuer"));
        }
        let code = params.code.ok_or_else(|| AppError::bad("missing authorization code"))?;
        let key = EsKey::from_hex(&pending.dpop_key)?;
        let mut nonce = pending.dpop_nonce;
        let token_url = Url::parse(&pending.token_endpoint).map_err(|e| AppError::bad(e.to_string()))?;
        let form = vec![
            ("grant_type".to_owned(), "authorization_code".to_owned()),
            ("code".to_owned(), code),
            ("redirect_uri".to_owned(), self.redirect_uri()),
            ("code_verifier".to_owned(), pending.pkce_verifier),
        ];
        let v = self.post_as(db, fetcher, &token_url, &pending.issuer, &form, Dpop { key: &key, nonce: &mut nonce }).await?;
        let tokens = validate_tokens(&v, &pending.did)?;
        sqlx::query(
            "INSERT INTO atproto_accounts (user_id, did, handle, pds_url, issuer, token_endpoint, revocation_endpoint,
                 scope, access_token, refresh_token, expires_at, dpop_key, as_nonce, pds_nonce, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?)
             ON CONFLICT (user_id) DO UPDATE SET did = excluded.did, handle = excluded.handle,
                 pds_url = excluded.pds_url, issuer = excluded.issuer, token_endpoint = excluded.token_endpoint,
                 revocation_endpoint = excluded.revocation_endpoint, scope = excluded.scope,
                 access_token = excluded.access_token, refresh_token = excluded.refresh_token,
                 expires_at = excluded.expires_at, dpop_key = excluded.dpop_key, as_nonce = excluded.as_nonce,
                 pds_nonce = NULL, created_at = excluded.created_at",
        )
        .bind(pending.user_id)
        .bind(&pending.did)
        .bind(&pending.handle)
        .bind(&pending.pds_url)
        .bind(&pending.issuer)
        .bind(&pending.token_endpoint)
        .bind(&pending.revocation_endpoint)
        .bind(&tokens.scope)
        .bind(&tokens.access)
        .bind(&tokens.refresh)
        .bind(tokens.expires_at)
        .bind(&pending.dpop_key)
        .bind(&nonce)
        .bind(now())
        .execute(db)
        .await?;
        Ok(pending.user_id)
    }

    pub async fn account(&self, db: &SqlitePool, user_id: i64) -> AppResult<Option<Account>> {
        Ok(sqlx::query_as("SELECT * FROM atproto_accounts WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(db)
            .await?)
    }

    /// Returns the account with a usable access token, refreshing if it expires soon or equals `stale`.
    async fn fresh_account(&self, db: &SqlitePool, fetcher: &Fetcher, user_id: i64, stale: Option<&str>) -> AppResult<Account> {
        let lock = self.user_lock(user_id);
        let _guard = lock.lock().await;
        let acct = self
            .account(db, user_id)
            .await?
            .ok_or_else(|| AppError::bad("connect a Bluesky account first (Settings → Bluesky)"))?;
        let is_stale = stale.is_some_and(|s| s == acct.access_token);
        if !is_stale && acct.expires_at > now() + REFRESH_MARGIN {
            return Ok(acct);
        }
        let refresh = acct
            .refresh_token
            .clone()
            .ok_or_else(|| AppError::bad("your Bluesky session expired; reconnect it in Settings"))?;
        let key = EsKey::from_hex(&acct.dpop_key)?;
        let mut nonce = acct.as_nonce.clone();
        let token_url = Url::parse(&acct.token_endpoint).map_err(|e| AppError::bad(e.to_string()))?;
        let form = vec![
            ("grant_type".to_owned(), "refresh_token".to_owned()),
            ("refresh_token".to_owned(), refresh),
        ];
        let v = self
            .post_as(db, fetcher, &token_url, &acct.issuer, &form, Dpop { key: &key, nonce: &mut nonce })
            .await
            .map_err(|e| AppError::bad(format!("could not refresh your Bluesky session ({e}); reconnect it in Settings")))?;
        let tokens = validate_tokens(&v, &acct.did)?;
        sqlx::query(
            "UPDATE atproto_accounts SET access_token = ?, refresh_token = COALESCE(?, refresh_token),
                 expires_at = ?, scope = ?, as_nonce = ? WHERE user_id = ?",
        )
        .bind(&tokens.access)
        .bind(&tokens.refresh)
        .bind(tokens.expires_at)
        .bind(&tokens.scope)
        .bind(&nonce)
        .bind(user_id)
        .execute(db)
        .await?;
        Ok(Account {
            access_token: tokens.access,
            refresh_token: tokens.refresh.or(acct.refresh_token.clone()),
            expires_at: tokens.expires_at,
            as_nonce: nonce,
            ..acct
        })
    }

    /// Calls an XRPC method on the user's PDS with their DPoP-bound token.
    pub async fn pds_call(
        &self,
        db: &SqlitePool,
        fetcher: &Fetcher,
        user_id: i64,
        method: Method,
        nsid: &str,
        body: Option<&Value>,
    ) -> AppResult<(Account, Value)> {
        let mut acct = self.fresh_account(db, fetcher, user_id, None).await?;
        let mut refreshed = false;
        for attempt in 0..4 {
            let pds = Url::parse(&acct.pds_url).map_err(|e| AppError::bad(e.to_string()))?;
            let url = pds.join(&format!("/xrpc/{nsid}")).map_err(|e| AppError::bad(e.to_string()))?;
            let key = EsKey::from_hex(&acct.dpop_key)?;
            let mut headers = HeaderMap::new();
            headers.insert("authorization", header_value(&format!("DPoP {}", acct.access_token))?);
            headers.insert(
                "dpop",
                header_value(&dpop_proof(&key, method.as_str(), &url, acct.pds_nonce.as_deref(), Some(&acct.access_token)))?,
            );
            let req_body = body.map_or(Body::Empty, Body::Json);
            let resp = fetcher
                .request(method.clone(), &url, headers, req_body, PDS_MAX_BYTES)
                .await
                .map_err(|e| AppError::bad(format!("{nsid}: {e}")))?;
            if let Some(n) = resp.header("dpop-nonce")
                && acct.pds_nonce.as_deref() != Some(n)
            {
                acct.pds_nonce = Some(n.to_owned());
                sqlx::query("UPDATE atproto_accounts SET pds_nonce = ? WHERE user_id = ?")
                    .bind(n)
                    .bind(user_id)
                    .execute(db)
                    .await?;
            }
            let json = resp.json().unwrap_or(Value::Null);
            if resp.status.is_success() {
                return Ok((acct, json));
            }
            let www = resp.header("www-authenticate").unwrap_or_default().to_owned();
            let unauthorized = resp.status == reqwest::StatusCode::UNAUTHORIZED;
            if unauthorized && www.contains("use_dpop_nonce") && attempt < 3 {
                continue;
            }
            let expired = www.contains("invalid_token") || matches!(json["error"].as_str(), Some("InvalidToken" | "ExpiredToken"));
            if unauthorized && expired && !refreshed {
                refreshed = true;
                acct = self.fresh_account(db, fetcher, user_id, Some(&acct.access_token)).await?;
                continue;
            }
            return Err(AppError::bad(format!(
                "{nsid}: HTTP {} {}",
                resp.status.as_u16(),
                json["message"].as_str().or(json["error"].as_str()).unwrap_or_default()
            )));
        }
        Err(AppError::bad(format!("{nsid}: PDS kept rejecting the request")))
    }

    pub async fn disconnect(&self, db: &SqlitePool, fetcher: &Fetcher, user_id: i64) -> AppResult<()> {
        if let Some(acct) = self.account(db, user_id).await?
            && let (Some(endpoint), Some(token)) = (acct.revocation_endpoint.as_deref(), acct.refresh_token.as_deref())
            && let Ok(url) = Url::parse(endpoint)
            && let Ok(key) = EsKey::from_hex(&acct.dpop_key)
        {
            let mut nonce = acct.as_nonce.clone();
            let form = vec![("token".to_owned(), token.to_owned())];
            if let Err(e) = self.post_as(db, fetcher, &url, &acct.issuer, &form, Dpop { key: &key, nonce: &mut nonce }).await {
                tracing::warn!(error = %e, "token revocation failed");
            }
        }
        sqlx::query("DELETE FROM atproto_accounts WHERE user_id = ?").bind(user_id).execute(db).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_is_consistent() {
        let a = Atproto::new(Url::parse("https://rss.example.com/sub").unwrap(), Network::default());
        let m = a.metadata();
        assert_eq!(m["client_id"], "https://rss.example.com/sub/oauth/client-metadata.json");
        assert_eq!(m["redirect_uris"][0], "https://rss.example.com/sub/oauth/callback");
        assert_eq!(m["jwks_uri"], "https://rss.example.com/sub/oauth/jwks.json");
        assert_eq!(m["dpop_bound_access_tokens"], true);
    }

    #[test]
    fn dpop_proof_binds_method_url_and_token() {
        let key = EsKey::generate();
        let url = Url::parse("https://pds.example.com/xrpc/com.atproto.repo.createRecord?x=1#f").unwrap();
        let jwt = dpop_proof(&key, "POST", &url, Some("n0nce"), Some("tok"));
        let claims: Value = serde_json::from_slice(
            &base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, jwt.split('.').nth(1).unwrap()).unwrap(),
        )
        .unwrap();
        assert_eq!(claims["htu"], "https://pds.example.com/xrpc/com.atproto.repo.createRecord");
        assert_eq!(claims["htm"], "POST");
        assert_eq!(claims["nonce"], "n0nce");
        assert_eq!(claims["ath"], b64(Sha256::digest(b"tok")));
    }

    #[test]
    fn token_validation() {
        let ok = json!({"token_type": "DPoP", "sub": "did:plc:a", "scope": "atproto transition:generic", "access_token": "t", "expires_in": 3600});
        assert!(validate_tokens(&ok, "did:plc:a").is_ok());
        assert!(validate_tokens(&ok, "did:plc:b").is_err());
        let bearer = json!({"token_type": "Bearer", "sub": "did:plc:a", "scope": "atproto", "access_token": "t"});
        assert!(validate_tokens(&bearer, "did:plc:a").is_err());
        let noscope = json!({"token_type": "DPoP", "sub": "did:plc:a", "scope": "transition:generic", "access_token": "t"});
        assert!(validate_tokens(&noscope, "did:plc:a").is_err());
    }
}
