use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;
use tower_http::compression::CompressionLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Assets;

const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; img-src * data: blob:; \
    media-src *; connect-src 'self'; font-src 'self'; manifest-src 'self'; \
    frame-src https://www.youtube-nocookie.com https://www.youtube.com https://player.vimeo.com; \
    form-action 'self'; base-uri 'none'; frame-ancestors 'none'";

pub fn app(state: AppState) -> Router {
    let api = crate::api::router()
        .merge(crate::atproto::routes::api_router())
        .nest("/greader", crate::greader::router())
        .nest("/greader.php", crate::greader::router());
    let headers: [(HeaderName, &'static str); 6] = [
        (header::CONTENT_SECURITY_POLICY, CSP),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::REFERRER_POLICY, "no-referrer"),
        (header::X_FRAME_OPTIONS, "DENY"),
        (HeaderName::from_static("cross-origin-opener-policy"), "same-origin"),
        (HeaderName::from_static("permissions-policy"), "camera=(), microphone=(), geolocation=(), interest-cohort=()"),
    ];
    let mut router = Router::new()
        .nest("/api", api)
        .merge(crate::atproto::routes::public_router())
        .fallback(static_handler)
        .with_state(state)
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024))
        .layer(CompressionLayer::new());
    for (name, value) in headers {
        router = router.layer(SetResponseHeaderLayer::if_not_present(name, HeaderValue::from_static(value)));
    }
    router.layer(TraceLayer::new_for_http())
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "ico" => "image/x-icon",
        "webmanifest" => "application/manifest+json",
        "json" => "application/json",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

async fn static_handler(uri: Uri, req_headers: HeaderMap) -> Response {
    let path = match uri.path().trim_start_matches('/') {
        "" => "index.html",
        p => p,
    };
    if path.starts_with("api/") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(file) = Assets::get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let etag = format!("\"{}\"", hex::encode(&file.metadata.sha256_hash()[..12]));
    let headers = [
        (header::CONTENT_TYPE, content_type(path).to_owned()),
        (header::ETAG, etag.clone()),
        (header::CACHE_CONTROL, "no-cache".to_owned()),
    ];
    let fresh = req_headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|t| t.trim() == etag));
    if fresh {
        return (StatusCode::NOT_MODIFIED, headers).into_response();
    }
    (headers, file.data.into_owned()).into_response()
}
