use reqwest::Method;
use reqwest::header::HeaderMap;
use serde_json::Value;
use url::Url;

use crate::error::{AppError, AppResult};
use crate::fetch::{Body, Fetcher};

const DOC_MAX_BYTES: usize = 256 * 1024;

/// Public services yesrss consults; overridable for tests.
#[derive(Clone, Debug)]
pub struct Network {
    pub plc: Url,
    pub appview: Url,
    pub relay: Url,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            plc: Url::parse("https://plc.directory").unwrap(),
            appview: Url::parse("https://public.api.bsky.app").unwrap(),
            relay: Url::parse("https://bsky.network").unwrap(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub did: String,
    pub handle: String,
    pub pds: Url,
}

pub fn is_did(s: &str) -> bool {
    let mut parts = s.splitn(3, ':');
    parts.next() == Some("did")
        && parts.next().is_some_and(|m| !m.is_empty() && m.chars().all(|c| c.is_ascii_lowercase()))
        && parts
            .next()
            .is_some_and(|id| !id.is_empty() && id.len() <= 2048 && id.chars().all(|c| c.is_ascii_alphanumeric() || "._:%-".contains(c)))
}

pub fn is_handle(s: &str) -> bool {
    let labels: Vec<&str> = s.split('.').collect();
    s.len() <= 253
        && labels.len() >= 2
        && labels.iter().all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
        && labels.last().is_some_and(|tld| tld.starts_with(|c: char| c.is_ascii_alphabetic()))
}

pub async fn get_json(fetcher: &Fetcher, url: &Url) -> AppResult<Value> {
    let resp = fetcher
        .request(Method::GET, url, HeaderMap::new(), Body::Empty, DOC_MAX_BYTES)
        .await
        .map_err(|e| AppError::bad(format!("{url}: {e}")))?;
    if !resp.status.is_success() {
        return Err(AppError::bad(format!("{url}: HTTP {}", resp.status)));
    }
    resp.json().ok_or_else(|| AppError::bad(format!("{url}: not JSON")))
}

async fn handle_to_did(fetcher: &Fetcher, net: &Network, handle: &str) -> AppResult<String> {
    if let Ok(url) = Url::parse(&format!("https://{handle}/.well-known/atproto-did"))
        && let Ok(resp) = fetcher.request(Method::GET, &url, HeaderMap::new(), Body::Empty, 4096).await
        && resp.status.is_success()
    {
        let did = String::from_utf8_lossy(&resp.body).trim().to_owned();
        if is_did(&did) {
            return Ok(did);
        }
    }
    let mut url = net.appview.join("/xrpc/com.atproto.identity.resolveHandle").unwrap();
    url.query_pairs_mut().append_pair("handle", handle);
    let v = get_json(fetcher, &url).await?;
    v["did"]
        .as_str()
        .filter(|d| is_did(d))
        .map(str::to_owned)
        .ok_or_else(|| AppError::bad(format!("could not resolve handle {handle}")))
}

fn did_doc_url(net: &Network, did: &str) -> AppResult<Url> {
    if did.starts_with("did:plc:") {
        return net.plc.join(&format!("/{did}")).map_err(|e| AppError::bad(e.to_string()));
    }
    if let Some(host) = did.strip_prefix("did:web:")
        && !host.contains(':')
    {
        return Url::parse(&format!("https://{host}/.well-known/did.json")).map_err(|e| AppError::bad(e.to_string()));
    }
    Err(AppError::bad(format!("unsupported DID method: {did}")))
}

pub fn parse_did_doc(did: &str, doc: &Value) -> AppResult<(Option<String>, Url)> {
    if doc["id"].as_str() != Some(did) {
        return Err(AppError::bad("DID document does not match the DID"));
    }
    let handle = doc["alsoKnownAs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .find_map(|aka| aka.strip_prefix("at://"))
        .map(str::to_ascii_lowercase);
    let pds = doc["service"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|s| {
            s["id"].as_str().is_some_and(|id| id == "#atproto_pds" || id == format!("{did}#atproto_pds"))
                && s["type"].as_str() == Some("AtprotoPersonalDataServer")
        })
        .and_then(|s| s["serviceEndpoint"].as_str())
        .and_then(|e| Url::parse(e).ok())
        .filter(|u| matches!(u.scheme(), "https" | "http"))
        .ok_or_else(|| AppError::bad("DID document has no PDS endpoint"))?;
    Ok((handle, pds))
}

pub async fn resolve_did(fetcher: &Fetcher, net: &Network, did: &str) -> AppResult<(Option<String>, Url)> {
    let doc = get_json(fetcher, &did_doc_url(net, did)?).await?;
    parse_did_doc(did, &doc)
}

/// Resolves a handle or DID to its account. A handle must be confirmed by the DID document.
pub async fn resolve(fetcher: &Fetcher, net: &Network, input: &str) -> AppResult<Identity> {
    let input = input.trim().trim_start_matches('@').to_ascii_lowercase();
    if is_did(&input) {
        let (handle, pds) = resolve_did(fetcher, net, &input).await?;
        return Ok(Identity { handle: handle.unwrap_or_else(|| input.clone()), did: input, pds });
    }
    if !is_handle(&input) {
        return Err(AppError::bad("enter a handle like alice.bsky.social or a DID"));
    }
    let did = handle_to_did(fetcher, net, &input).await?;
    let (doc_handle, pds) = resolve_did(fetcher, net, &did).await?;
    if doc_handle.as_deref() != Some(input.as_str()) {
        return Err(AppError::bad(format!("{input} is not confirmed by its DID document")));
    }
    Ok(Identity { did, handle: input, pds })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn validates_syntax() {
        assert!(is_did("did:plc:re3ebnp5v7ffagz6rb6xfei4"));
        assert!(is_did("did:web:example.com"));
        assert!(!is_did("did:PLC:x"));
        assert!(!is_did("did:plc:"));
        assert!(!is_did("did:plc:a/b"));
        assert!(is_handle("alice.bsky.social"));
        assert!(!is_handle("localhost"));
        assert!(!is_handle("-bad.example.com"));
        assert!(!is_handle("a.123"));
        assert!(!is_handle("a..b"));
    }

    #[test]
    fn parses_did_documents() {
        let doc = json!({
            "id": "did:plc:abc",
            "alsoKnownAs": ["at://Alice.Example.com"],
            "service": [{"id": "#atproto_pds", "type": "AtprotoPersonalDataServer", "serviceEndpoint": "https://pds.example.com"}]
        });
        let (handle, pds) = parse_did_doc("did:plc:abc", &doc).unwrap();
        assert_eq!(handle.as_deref(), Some("alice.example.com"));
        assert_eq!(pds.as_str(), "https://pds.example.com/");
        assert!(parse_did_doc("did:plc:other", &doc).is_err());
        assert!(parse_did_doc("did:plc:abc", &json!({"id": "did:plc:abc", "service": []})).is_err());
    }
}
