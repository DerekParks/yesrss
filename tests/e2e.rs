use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use wiremock::matchers::{header as req_header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use yesrss::config::Config;
use yesrss::state::AppState;

const RSS: &str = r#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>Test Blog</title><link>https://blog.example/</link>
<item><title>Hello world</title><link>https://blog.example/hello</link><guid>hello</guid>
<description>&lt;p&gt;Greetings &lt;script&gt;steal()&lt;/script&gt;from the test suite&lt;/p&gt;</description>
<pubDate>Mon, 01 Sep 2025 10:00:00 GMT</pubDate></item>
<item><title>Second post</title><link>https://blog.example/second</link><guid>second</guid>
<description>Another one about zebras</description>
<pubDate>Tue, 02 Sep 2025 10:00:00 GMT</pubDate></item>
</channel></rss>"#;

struct TestApp {
    app: Router,
    state: AppState,
    _dir: tempfile::TempDir,
}

struct Resp {
    status: StatusCode,
    headers: HeaderMap,
    text: String,
}

impl Resp {
    fn json(&self) -> Value {
        serde_json::from_str(&self.text).unwrap_or_else(|e| panic!("{e}: {}", self.text))
    }
}

impl TestApp {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = yesrss::db::connect(&dir.path().join("test.db")).await.unwrap();
        let cfg = Config { allow_private_networks: true, ..Config::default() };
        let state = AppState::new(db, cfg).unwrap();
        Self { app: yesrss::web::app(state.clone()), state, _dir: dir }
    }

    async fn call(&self, req: Request<Body>) -> Resp {
        let res = self.app.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let headers = res.headers().clone();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        Resp { status, headers, text: String::from_utf8_lossy(&bytes).into_owned() }
    }

    async fn web(&self, m: Method, uri: &str, cookie: Option<&str>, body: Option<Value>) -> Resp {
        let mut b = Request::builder().method(m).uri(uri).header("x-yesrss", "1");
        if let Some(c) = cookie {
            b = b.header(header::COOKIE, c);
        }
        let req = match body {
            Some(v) => b.header(header::CONTENT_TYPE, "application/json").body(Body::from(v.to_string())),
            None => b.body(Body::empty()),
        };
        self.call(req.unwrap()).await
    }

    async fn greader(&self, m: Method, uri: &str, auth: &str, form: &str) -> Resp {
        let req = Request::builder()
            .method(m)
            .uri(format!("/api/greader{uri}"))
            .header(header::AUTHORIZATION, format!("GoogleLogin auth={auth}"))
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(form.to_owned()))
            .unwrap();
        self.call(req).await
    }
}

fn session_cookie(r: &Resp) -> String {
    let set = r.headers.get(header::SET_COOKIE).expect("set-cookie").to_str().unwrap();
    assert!(set.contains("HttpOnly") && set.contains("SameSite=Strict"), "{set}");
    set.split(';').next().unwrap().to_owned()
}

async fn feed_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/rss+xml")
                .insert_header("etag", "\"v1\"")
                .set_body_string(RSS),
        )
        .mount(&server)
        .await;
    let html = format!(
        r#"<html><head><link rel="alternate" type="application/rss+xml" title="Test Blog" href="{}/feed.xml"></head></html>"#,
        server.uri()
    );
    Mock::given(method("GET"))
        .and(path("/blog"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "text/html").set_body_string(html))
        .mount(&server)
        .await;
    server
}

async fn setup_user(t: &TestApp) -> String {
    let r = t.web(Method::POST, "/api/setup", None, Some(json!({"username": "reader", "password": "hunter2hunter2"}))).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text);
    session_cookie(&r)
}

#[tokio::test]
async fn setup_login_and_csrf() {
    let t = TestApp::new().await;
    let r = t.web(Method::GET, "/api/setup", None, None).await;
    assert_eq!(r.json()["needed"], true);

    let no_csrf = Request::builder()
        .method(Method::POST)
        .uri("/api/setup")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({"username": "x", "password": "longenough"}).to_string()))
        .unwrap();
    assert_eq!(t.call(no_csrf).await.status, StatusCode::FORBIDDEN);

    let r = t.web(Method::POST, "/api/setup", None, Some(json!({"username": "reader", "password": "short"}))).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);

    let cookie = setup_user(&t).await;
    let r = t.web(Method::POST, "/api/setup", None, Some(json!({"username": "evil", "password": "hunter2hunter2"}))).await;
    assert_eq!(r.status, StatusCode::CONFLICT);

    assert_eq!(t.web(Method::GET, "/api/session", Some(&cookie), None).await.json()["user"]["username"], "reader");
    assert_eq!(t.web(Method::GET, "/api/tree", None, None).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(t.web(Method::GET, "/api/tree", Some("yesrss_session=forged"), None).await.status, StatusCode::UNAUTHORIZED);

    let bad = t.web(Method::POST, "/api/login", None, Some(json!({"username": "reader", "password": "wrong-password"}))).await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
    let good = t.web(Method::POST, "/api/login", None, Some(json!({"username": "READER", "password": "hunter2hunter2"}))).await;
    assert_eq!(good.status, StatusCode::OK);
    let cookie2 = session_cookie(&good);

    let r = t.web(Method::POST, "/api/logout", Some(&cookie2), None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(t.web(Method::GET, "/api/tree", Some(&cookie2), None).await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(t.web(Method::GET, "/api/tree", Some(&cookie), None).await.status, StatusCode::OK);

    let r = t.web(Method::POST, "/api/password", Some(&cookie), Some(json!({"current": "hunter2hunter2", "new": "correct horse battery"}))).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(t.web(Method::GET, "/api/tree", Some(&cookie), None).await.status, StatusCode::UNAUTHORIZED);
    let fresh = session_cookie(&r);
    assert_eq!(t.web(Method::GET, "/api/tree", Some(&fresh), None).await.status, StatusCode::OK);
}

#[tokio::test]
async fn login_is_rate_limited() {
    let t = TestApp::new().await;
    setup_user(&t).await;
    for _ in 0..10 {
        let r = t.web(Method::POST, "/api/login", None, Some(json!({"username": "reader", "password": "nope-nope"}))).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST);
    }
    let r = t.web(Method::POST, "/api/login", None, Some(json!({"username": "reader", "password": "hunter2hunter2"}))).await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn subscribe_read_star_search() {
    let server = feed_server().await;
    let t = TestApp::new().await;
    let cookie = setup_user(&t).await;
    let c = Some(cookie.as_str());

    let r = t.web(Method::POST, "/api/discover", c, Some(json!({"url": format!("{}/blog", server.uri())}))).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text);
    assert_eq!(r.json()["candidates"][0]["url"], format!("{}/feed.xml", server.uri()));

    let r = t.web(Method::POST, "/api/feeds", c, Some(json!({"url": format!("{}/blog", server.uri()), "folder_name": "Tech"}))).await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text);
    let feed_id = r.json()["id"].as_i64().unwrap();

    let dup = t.web(Method::POST, "/api/feeds", c, Some(json!({"url": format!("{}/feed.xml", server.uri())}))).await;
    assert_eq!(dup.status, StatusCode::CONFLICT);

    let tree = t.web(Method::GET, "/api/tree", c, None).await.json();
    assert_eq!(tree["unread"], 2);
    assert_eq!(tree["folders"][0]["name"], "Tech");
    assert_eq!(tree["feeds"][0]["title"], "Test Blog");
    assert_eq!(tree["feeds"][0]["folder_id"], tree["folders"][0]["id"]);

    let list = t.web(Method::GET, "/api/entries?unread=true", c, None).await.json();
    let entries = list["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["title"], "Second post");
    let hello = &entries[1];
    assert!(!hello["content"].as_str().unwrap().contains("script"));
    let hello_id = hello["id"].as_i64().unwrap();

    let page = t.web(Method::GET, "/api/entries?limit=1", c, None).await.json();
    assert_eq!(page["entries"].as_array().unwrap().len(), 1);
    let next = page["next"].as_str().unwrap();
    let page2 = t.web(Method::GET, &format!("/api/entries?limit=1&cursor={next}"), c, None).await.json();
    assert_eq!(page2["entries"][0]["id"], hello_id);
    assert!(page2["next"].is_null());

    let r = t.web(Method::PATCH, &format!("/api/entries/{hello_id}"), c, Some(json!({"is_read": true, "is_starred": true}))).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let tree = t.web(Method::GET, "/api/tree", c, None).await.json();
    assert_eq!(tree["unread"], 1);
    assert_eq!(tree["starred"], 1);
    let starred = t.web(Method::GET, "/api/entries?starred=true", c, None).await.json();
    assert_eq!(starred["entries"][0]["id"], hello_id);

    let found = t.web(Method::GET, "/api/entries?q=zebra", c, None).await.json();
    assert_eq!(found["entries"].as_array().unwrap().len(), 1);
    assert_eq!(found["entries"][0]["title"], "Second post");
    let weird = t.web(Method::GET, "/api/entries?q=%22%29%20OR%20*", c, None).await;
    assert_eq!(weird.status, StatusCode::OK, "{}", weird.text);

    let r = t.web(Method::POST, "/api/entries/mark-read", c, Some(json!({"feed": feed_id}))).await;
    assert_eq!(r.json()["marked"], 1);
    assert_eq!(t.web(Method::GET, "/api/tree", c, None).await.json()["unread"], 0);

    Mock::given(method("GET"))
        .and(path("/feed.xml"))
        .and(req_header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304))
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    let r = t.web(Method::POST, &format!("/api/feeds/{feed_id}/refresh"), c, None).await;
    assert_eq!(r.json()["new"], 0);

    let r = t.web(Method::PATCH, &format!("/api/feeds/{feed_id}"), c, Some(json!({"title": "Renamed", "folder_id": null}))).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let tree = t.web(Method::GET, "/api/tree", c, None).await.json();
    assert_eq!(tree["feeds"][0]["title"], "Renamed");
    assert!(tree["feeds"][0]["folder_id"].is_null());

    let other = {
        let token = yesrss::auth::create_user(&t.state.db, "mallory", "mallorypassword").await.unwrap();
        let s = yesrss::auth::create_session(&t.state.db, token.id, yesrss::auth::SessionKind::Web).await.unwrap();
        format!("yesrss_session={s}")
    };
    let r = t.web(Method::DELETE, &format!("/api/feeds/{feed_id}"), Some(&other), None).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let r = t.web(Method::PATCH, &format!("/api/entries/{hello_id}"), Some(&other), Some(json!({"is_read": false}))).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(t.web(Method::GET, "/api/tree", c, None).await.json()["unread"], 0);
    assert_eq!(t.web(Method::GET, "/api/entries", Some(&other), None).await.json()["entries"].as_array().unwrap().len(), 0);

    let r = t.web(Method::DELETE, &format!("/api/feeds/{feed_id}"), c, None).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(t.web(Method::GET, "/api/entries", c, None).await.json()["entries"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn opml_roundtrip() {
    let server = feed_server().await;
    let t = TestApp::new().await;
    let cookie = setup_user(&t).await;
    let c = Some(cookie.as_str());
    t.web(Method::POST, "/api/feeds", c, Some(json!({"url": format!("{}/feed.xml", server.uri()), "folder_name": "News & Stuff"}))).await;

    let r = t.web(Method::GET, "/api/opml", c, None).await;
    assert!(r.text.contains("News &amp; Stuff"), "{}", r.text);
    assert!(r.text.contains(&format!("{}/feed.xml", server.uri())));

    let opml = r#"<opml version="2.0"><body><outline text="Misc"><outline xmlUrl="https://a.example/rss" text="A"/></outline>
<outline xmlUrl="javascript:alert(1)" text="bad"/><outline xmlUrl="https://b.example/rss" text="B"/></body></opml>"#;
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/opml")
        .header("x-yesrss", "1")
        .header(header::COOKIE, cookie.as_str())
        .body(Body::from(opml))
        .unwrap();
    let r = t.call(req).await.json();
    assert_eq!(r["added"], 2);
    assert_eq!(r["skipped"], 1);
    let tree = t.web(Method::GET, "/api/tree", c, None).await.json();
    assert_eq!(tree["feeds"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn google_reader_api() {
    let server = feed_server().await;
    let t = TestApp::new().await;
    setup_user(&t).await;

    let r = t.greader(Method::POST, "/accounts/ClientLogin", "", "Email=reader&Passwd=wrong").await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    let r = t.greader(Method::POST, "/accounts/ClientLogin", "", "Email=reader&Passwd=hunter2hunter2").await;
    assert_eq!(r.status, StatusCode::OK, "{}", r.text);
    let auth = r.text.lines().find_map(|l| l.strip_prefix("Auth=")).unwrap().to_owned();

    assert_eq!(t.greader(Method::GET, "/reader/api/0/user-info", "bogus", "").await.status, StatusCode::UNAUTHORIZED);
    let info = t.greader(Method::GET, "/reader/api/0/user-info", &auth, "").await.json();
    assert_eq!(info["userName"], "reader");
    assert!(!t.greader(Method::GET, "/reader/api/0/token", &auth, "").await.text.is_empty());

    let feed_url = format!("{}/feed.xml", server.uri());
    let form = format!(
        "ac=subscribe&s=feed%2F{}&a=user%2F-%2Flabel%2FTech&t=My+Feed",
        url::form_urlencoded::byte_serialize(feed_url.as_bytes()).collect::<String>()
    );
    let r = t.greader(Method::POST, "/reader/api/0/subscription/edit", &auth, &form).await;
    assert_eq!(r.text, "OK");

    let subs = t.greader(Method::GET, "/reader/api/0/subscription/list?output=json", &auth, "").await.json();
    let sub = &subs["subscriptions"][0];
    assert_eq!(sub["title"], "My Feed");
    assert_eq!(sub["categories"][0]["id"], "user/-/label/Tech");
    let stream = sub["id"].as_str().unwrap().to_owned();

    let tags = t.greader(Method::GET, "/reader/api/0/tag/list", &auth, "").await.json();
    assert!(tags["tags"].as_array().unwrap().iter().any(|t| t["id"] == "user/-/label/Tech"));

    let counts = t.greader(Method::GET, "/reader/api/0/unread-count", &auth, "").await.json();
    assert_eq!(counts["max"], 2);

    let ids = t
        .greader(Method::GET, "/reader/api/0/stream/items/ids?s=user/-/state/com.google/reading-list&xt=user/-/state/com.google/read&n=1", &auth, "")
        .await
        .json();
    assert_eq!(ids["itemRefs"].as_array().unwrap().len(), 1);
    let first_id: i64 = ids["itemRefs"][0]["id"].as_str().unwrap().parse().unwrap();
    let cont = ids["continuation"].as_str().unwrap();
    let ids2 = t
        .greader(Method::GET, &format!("/reader/api/0/stream/items/ids?s=user/-/state/com.google/reading-list&n=1&c={cont}"), &auth, "")
        .await
        .json();
    let second_id: i64 = ids2["itemRefs"][0]["id"].as_str().unwrap().parse().unwrap();
    assert_ne!(first_id, second_id);

    let contents = t
        .greader(Method::POST, "/reader/api/0/stream/items/contents", &auth, &format!("i={first_id}&i=tag:google.com,2005:reader/item/{second_id:016x}"))
        .await
        .json();
    assert_eq!(contents["items"].as_array().unwrap().len(), 2);
    let item = &contents["items"][0];
    assert!(item["id"].as_str().unwrap().starts_with("tag:google.com,2005:reader/item/"));
    assert_eq!(item["origin"]["streamId"], stream);
    assert!(item["categories"].as_array().unwrap().iter().any(|c| c == "user/-/label/Tech"));

    let r = t
        .greader(Method::POST, "/reader/api/0/edit-tag", &auth, &format!("i={first_id}&a=user/-/state/com.google/read&a=user/-/state/com.google/starred"))
        .await;
    assert_eq!(r.text, "OK");
    let counts = t.greader(Method::GET, "/reader/api/0/unread-count", &auth, "").await.json();
    assert_eq!(counts["max"], 1);
    let starred = t.greader(Method::GET, "/reader/api/0/stream/contents/user/-/state/com.google/starred", &auth, "").await.json();
    assert_eq!(starred["items"].as_array().unwrap().len(), 1);
    let unread = t
        .greader(Method::GET, &format!("/reader/api/0/stream/contents/{stream}?xt=user/-/state/com.google/read"), &auth, "")
        .await
        .json();
    assert_eq!(unread["items"].as_array().unwrap().len(), 1);
    assert_eq!(unread["items"][0]["id"], format!("tag:google.com,2005:reader/item/{second_id:016x}"));

    let r = t.greader(Method::POST, "/reader/api/0/mark-all-as-read", &auth, "s=user/-/label/Tech").await;
    assert_eq!(r.text, "OK");
    let counts = t.greader(Method::GET, "/reader/api/0/unread-count", &auth, "").await.json();
    assert_eq!(counts["max"], 0);

    let r = t.greader(Method::POST, "/reader/api/0/rename-tag", &auth, "s=user/-/label/Tech&dest=user/-/label/Code").await;
    assert_eq!(r.text, "OK");
    let subs = t.greader(Method::GET, "/reader/api/0/subscription/list", &auth, "").await.json();
    assert_eq!(subs["subscriptions"][0]["categories"][0]["label"], "Code");

    let r = t.greader(Method::POST, "/reader/api/0/subscription/edit", &auth, &format!("ac=unsubscribe&s={stream}")).await;
    assert_eq!(r.text, "OK");
    let subs = t.greader(Method::GET, "/reader/api/0/subscription/list", &auth, "").await.json();
    assert_eq!(subs["subscriptions"].as_array().unwrap().len(), 0);

    let php = Request::builder()
        .uri("/api/greader.php/reader/api/0/user-info")
        .header(header::AUTHORIZATION, format!("GoogleLogin auth={auth}"))
        .body(Body::empty())
        .unwrap();
    assert_eq!(t.call(php).await.status, StatusCode::OK);
}

#[tokio::test]
async fn ssrf_guard_blocks_private_feeds_by_default() {
    let server = feed_server().await;
    let dir = tempfile::tempdir().unwrap();
    let db = yesrss::db::connect(&dir.path().join("t.db")).await.unwrap();
    let state = AppState::new(db, Config::default()).unwrap();
    let t = TestApp { app: yesrss::web::app(state.clone()), state, _dir: dir };
    let c = setup_user(&t).await;
    let r = t.web(Method::POST, "/api/feeds", Some(&c), Some(json!({"url": format!("{}/feed.xml", server.uri())}))).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert!(r.text.contains("non-public"), "{}", r.text);
}

#[tokio::test]
async fn serves_ui_with_security_headers() {
    let t = TestApp::new().await;
    let r = t.call(Request::builder().uri("/").body(Body::empty()).unwrap()).await;
    assert_eq!(r.status, StatusCode::OK);
    let csp = r.headers.get(header::CONTENT_SECURITY_POLICY).unwrap().to_str().unwrap();
    assert!(csp.contains("script-src 'self'") && csp.contains("frame-ancestors 'none'"));
    assert_eq!(r.headers.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(), "nosniff");
    let etag = r.headers.get(header::ETAG).unwrap().clone();
    let again = Request::builder().uri("/").header(header::IF_NONE_MATCH, etag).body(Body::empty()).unwrap();
    assert_eq!(t.call(again).await.status, StatusCode::NOT_MODIFIED);
    let missing = t.call(Request::builder().uri("/../Cargo.toml").body(Body::empty()).unwrap()).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn opml_custom_titles_survive_first_fetch() {
    let server = feed_server().await;
    let t = TestApp::new().await;
    let cookie = setup_user(&t).await;
    let c = Some(cookie.as_str());
    let opml = format!(
        r#"<opml version="2.0"><body><outline xmlUrl="{0}/feed.xml" text="My Name For It"/><outline xmlUrl="{0}/feed.xml?plain"/></body></opml>"#,
        server.uri()
    );
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/opml")
        .header("x-yesrss", "1")
        .header(header::COOKIE, cookie.as_str())
        .body(Body::from(opml))
        .unwrap();
    assert_eq!(t.call(req).await.json()["added"], 2);
    let tree = t.web(Method::GET, "/api/tree", c, None).await.json();
    for f in tree["feeds"].as_array().unwrap() {
        let r = t.web(Method::POST, &format!("/api/feeds/{}/refresh", f["id"]), c, None).await;
        assert_eq!(r.json()["new"], 2);
    }
    let tree = t.web(Method::GET, "/api/tree", c, None).await.json();
    let mut titles: Vec<&str> = tree["feeds"].as_array().unwrap().iter().map(|f| f["title"].as_str().unwrap()).collect();
    titles.sort();
    assert_eq!(titles, ["My Name For It", "Test Blog"]);
}

#[tokio::test]
async fn backfill_after_mark_all_read_arrives_read() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/late.xml"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/late.xml"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "application/rss+xml").set_body_string(RSS))
        .mount(&server)
        .await;
    let t = TestApp::new().await;
    let cookie = setup_user(&t).await;
    let c = Some(cookie.as_str());
    let opml = format!(r#"<opml version="2.0"><body><outline xmlUrl="{}/late.xml"/></body></opml>"#, server.uri());
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/opml")
        .header("x-yesrss", "1")
        .header(header::COOKIE, cookie.as_str())
        .body(Body::from(opml))
        .unwrap();
    assert_eq!(t.call(req).await.json()["added"], 1);
    let feed = t.web(Method::GET, "/api/tree", c, None).await.json()["feeds"][0]["id"].as_i64().unwrap();
    let refresh = format!("/api/feeds/{feed}/refresh");

    assert_eq!(t.web(Method::POST, &refresh, c, None).await.status, StatusCode::BAD_REQUEST);
    let r = t.web(Method::POST, "/api/entries/mark-read", c, Some(json!({}))).await;
    assert_eq!(r.json()["marked"], 0);

    assert_eq!(t.web(Method::POST, &refresh, c, None).await.json()["new"], 2);
    assert_eq!(t.web(Method::GET, "/api/tree", c, None).await.json()["unread"], 0);

    let late_item = RSS.replace(
        "</channel>",
        "<item><title>Backdated</title><guid>backdated</guid><pubDate>Wed, 03 Sep 2025 10:00:00 GMT</pubDate></item></channel>",
    );
    Mock::given(method("GET"))
        .and(path("/late.xml"))
        .respond_with(ResponseTemplate::new(200).insert_header("content-type", "application/rss+xml").set_body_string(late_item))
        .with_priority(1)
        .mount(&server)
        .await;
    assert_eq!(t.web(Method::POST, &refresh, c, None).await.json()["new"], 1);
    assert_eq!(t.web(Method::GET, "/api/tree", c, None).await.json()["unread"], 1);
}

mod atproto_flow {
    use std::sync::{Arc, Mutex};

    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use p256::ecdsa::signature::Verifier;
    use p256::ecdsa::{Signature, VerifyingKey};
    use wiremock::{Request as MockRequest, Respond};
    use yesrss::atproto::identity::Network;

    use super::*;

    const DID: &str = "did:plc:alice";
    const AUTHOR: &str = "did:plc:author";

    fn jwt_part(jwt: &str, i: usize) -> Value {
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(jwt.split('.').nth(i).unwrap()).unwrap()).unwrap()
    }

    /// Verifies a DPoP proof's signature against its embedded key and returns its claims.
    fn verified_dpop(req: &MockRequest) -> Value {
        let jwt = req.headers.get("dpop").expect("DPoP header").to_str().unwrap();
        let header = jwt_part(jwt, 0);
        assert_eq!(header["typ"], "dpop+jwt");
        assert_eq!(header["alg"], "ES256");
        let mut sec1 = vec![4u8];
        for c in ["x", "y"] {
            sec1.extend(URL_SAFE_NO_PAD.decode(header["jwk"][c].as_str().unwrap()).unwrap());
        }
        let vk = VerifyingKey::from_sec1_bytes(&sec1).unwrap();
        let (input, sig) = jwt.rsplit_once('.').unwrap();
        let sig = Signature::from_slice(&URL_SAFE_NO_PAD.decode(sig).unwrap()).unwrap();
        vk.verify(input.as_bytes(), &sig).expect("valid DPoP signature");
        let claims = jwt_part(jwt, 1);
        assert_eq!(claims["htm"], req.method.as_str());
        let htu = url::Url::parse(claims["htu"].as_str().unwrap()).unwrap();
        assert_eq!(htu.path(), req.url.path());
        assert!(htu.query().is_none() && htu.fragment().is_none());
        claims
    }

    fn form(req: &MockRequest) -> std::collections::HashMap<String, String> {
        url::form_urlencoded::parse(&req.body).into_owned().collect()
    }

    #[derive(Clone, Default)]
    struct Seen {
        state: Arc<Mutex<Option<String>>>,
        records: Arc<Mutex<Vec<Value>>>,
        deleted: Arc<Mutex<Vec<Value>>>,
    }

    struct AuthServer {
        seen: Seen,
        issuer: String,
    }

    impl Respond for AuthServer {
        fn respond(&self, req: &MockRequest) -> ResponseTemplate {
            let claims = verified_dpop(req);
            if claims["nonce"] != "as-nonce-1" {
                return ResponseTemplate::new(400)
                    .insert_header("dpop-nonce", "as-nonce-1")
                    .set_body_json(json!({"error": "use_dpop_nonce"}));
            }
            let f = form(req);
            assert_eq!(f["client_id"], "https://rss.test/oauth/client-metadata.json");
            assert_eq!(f["client_assertion_type"], "urn:ietf:params:oauth:client-assertion-type:jwt-bearer");
            let assertion = jwt_part(&f["client_assertion"], 1);
            assert_eq!(assertion["aud"], self.issuer);
            assert_eq!(assertion["iss"], f["client_id"]);
            match req.url.path() {
                "/oauth/par" => {
                    assert_eq!(f["code_challenge_method"], "S256");
                    assert_eq!(f["redirect_uri"], "https://rss.test/oauth/callback");
                    *self.seen.state.lock().unwrap() = Some(f["state"].clone());
                    ResponseTemplate::new(201).set_body_json(json!({"request_uri": "urn:ietf:params:oauth:request_uri:abc", "expires_in": 60}))
                }
                "/oauth/token" => {
                    let access = match f["grant_type"].as_str() {
                        "authorization_code" => {
                            assert_eq!(f["code"], "the-code");
                            assert!(f["code_verifier"].len() >= 43);
                            "at-1"
                        }
                        "refresh_token" => "at-2",
                        other => panic!("unexpected grant {other}"),
                    };
                    ResponseTemplate::new(200).set_body_json(json!({
                        "access_token": access, "refresh_token": "rt-1", "token_type": "DPoP",
                        "sub": DID, "scope": "atproto transition:generic", "expires_in": 3600
                    }))
                }
                "/oauth/revoke" => ResponseTemplate::new(200),
                other => panic!("unexpected AS path {other}"),
            }
        }
    }

    struct Pds {
        seen: Seen,
    }

    impl Respond for Pds {
        fn respond(&self, req: &MockRequest) -> ResponseTemplate {
            let claims = verified_dpop(req);
            if claims["nonce"] != "pds-nonce-1" {
                return ResponseTemplate::new(401)
                    .insert_header("dpop-nonce", "pds-nonce-1")
                    .insert_header("www-authenticate", r#"DPoP error="use_dpop_nonce""#);
            }
            let auth = req.headers.get("authorization").unwrap().to_str().unwrap();
            let token = auth.strip_prefix("DPoP ").expect("DPoP auth scheme");
            let ath = URL_SAFE_NO_PAD.encode(<sha2::Sha256 as sha2::Digest>::digest(token.as_bytes()));
            assert_eq!(claims["ath"], ath);
            let body: Value = serde_json::from_slice(&req.body).unwrap();
            assert_eq!(body["repo"], DID);
            if req.url.path().ends_with("createRecord") {
                let n = {
                    let mut r = self.seen.records.lock().unwrap();
                    r.push(body.clone());
                    r.len()
                };
                ResponseTemplate::new(200)
                    .set_body_json(json!({"uri": format!("at://{DID}/{}/rk{n}", body["collection"].as_str().unwrap()), "cid": "bafy"}))
            } else {
                self.seen.deleted.lock().unwrap().push(body);
                ResponseTemplate::new(200).set_body_json(json!({}))
            }
        }
    }

    fn did_doc(did: &str, handle: &str, pds: &str) -> Value {
        json!({
            "id": did,
            "alsoKnownAs": [format!("at://{handle}")],
            "service": [{"id": "#atproto_pds", "type": "AtprotoPersonalDataServer", "serviceEndpoint": pds}]
        })
    }

    async fn network() -> (MockServer, Seen) {
        let server = MockServer::start().await;
        let uri = server.uri();
        let seen = Seen::default();
        let ok_json = |v: Value| ResponseTemplate::new(200).set_body_json(v);
        Mock::given(path(format!("/{DID}"))).respond_with(ok_json(did_doc(DID, "alice.test", &uri))).mount(&server).await;
        Mock::given(path(format!("/{AUTHOR}"))).respond_with(ok_json(did_doc(AUTHOR, "author.test", &uri))).mount(&server).await;
        Mock::given(path("/xrpc/com.atproto.identity.resolveHandle"))
            .respond_with(ok_json(json!({"did": DID})))
            .mount(&server)
            .await;
        Mock::given(path("/.well-known/oauth-protected-resource"))
            .respond_with(ok_json(json!({"resource": uri, "authorization_servers": [uri]})))
            .mount(&server)
            .await;
        Mock::given(path("/.well-known/oauth-authorization-server"))
            .respond_with(ok_json(json!({
                "issuer": uri,
                "authorization_endpoint": format!("{uri}/oauth/authorize"),
                "token_endpoint": format!("{uri}/oauth/token"),
                "pushed_authorization_request_endpoint": format!("{uri}/oauth/par"),
                "revocation_endpoint": format!("{uri}/oauth/revoke"),
                "token_endpoint_auth_methods_supported": ["none", "private_key_jwt"],
                "dpop_signing_alg_values_supported": ["ES256"],
            })))
            .mount(&server)
            .await;
        for p in ["/oauth/par", "/oauth/token", "/oauth/revoke"] {
            Mock::given(method("POST"))
                .and(path(p))
                .respond_with(AuthServer { seen: seen.clone(), issuer: uri.clone() })
                .mount(&server)
                .await;
        }
        for p in ["/xrpc/com.atproto.repo.createRecord", "/xrpc/com.atproto.repo.deleteRecord"] {
            Mock::given(method("POST")).and(path(p)).respond_with(Pds { seen: seen.clone() }).mount(&server).await;
        }
        Mock::given(path("/xrpc/com.atproto.repo.getRecord")).respond_with(ok_json(json!({"value": {}}))).mount(&server).await;
        let rss = format!(
            r#"<rss version="2.0"><channel><title>Std</title><item><title>Doc post</title><link>{uri}/post1</link><guid>p1</guid></item>
<item><title>Plain post</title><link>{uri}/post2</link><guid>p2</guid></item></channel></rss>"#
        );
        Mock::given(path("/feed.xml")).respond_with(ResponseTemplate::new(200).set_body_string(rss)).mount(&server).await;
        Mock::given(path("/post1"))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                r#"<html><head><link rel="site.standard.document" href="at://{AUTHOR}/site.standard.document/doc1"></head></html>"#
            )))
            .mount(&server)
            .await;
        Mock::given(path("/post2")).respond_with(ResponseTemplate::new(200).set_body_string("<html></html>")).mount(&server).await;
        Mock::given(path("/xrpc/app.bsky.graph.getFollows"))
            .respond_with(ok_json(json!({"follows": [{"did": AUTHOR, "handle": "author.test"}, {"did": "did:plc:quiet", "handle": "quiet.test"}]})))
            .mount(&server)
            .await;
        Mock::given(path("/xrpc/com.atproto.sync.listReposByCollection"))
            .respond_with(ok_json(json!({"repos": [{"did": AUTHOR}, {"did": "did:plc:stranger"}]})))
            .mount(&server)
            .await;
        Mock::given(path("/xrpc/com.atproto.repo.listRecords"))
            .respond_with(ok_json(json!({"records": [{"uri": format!("at://{AUTHOR}/site.standard.publication/self"),
                "value": {"url": format!("{uri}/"), "name": "Author's Notes", "description": "notes"}}]})))
            .mount(&server)
            .await;
        (server, seen)
    }

    async fn app(server: &MockServer) -> TestApp {
        let dir = tempfile::tempdir().unwrap();
        let db = yesrss::db::connect(&dir.path().join("t.db")).await.unwrap();
        let cfg = Config {
            allow_private_networks: true,
            public_url: Some(url::Url::parse("https://rss.test").unwrap()),
            ..Config::default()
        };
        let base = url::Url::parse(&server.uri()).unwrap();
        let net = Network { plc: base.clone(), appview: base.clone(), relay: base };
        let state = AppState::with_network(db, cfg, net).unwrap();
        TestApp { app: yesrss::web::app(state.clone()), state, _dir: dir }
    }

    async fn connect(t: &TestApp, seen: &Seen, cookie: &str, handle: &str) -> (Resp, String) {
        let r = t.web(Method::POST, "/api/atproto/connect", Some(cookie), Some(json!({"handle": handle}))).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.text);
        let redirect = r.json()["redirect"].as_str().unwrap().to_owned();
        assert!(redirect.contains("/oauth/authorize?client_id="), "{redirect}");
        let set = r.headers.get(header::SET_COOKIE).unwrap().to_str().unwrap().to_owned();
        assert!(set.contains("SameSite=Lax") && set.contains("Path=/oauth/callback"), "{set}");
        let browser = set.split(';').next().unwrap().to_owned();
        let state = seen.state.lock().unwrap().clone().unwrap();
        (r, format!("{browser}|{state}"))
    }

    async fn callback(t: &TestApp, server: &MockServer, browser_and_state: &str, with_cookie: bool) -> Resp {
        let (browser, state) = browser_and_state.split_once('|').unwrap();
        let iss = url::form_urlencoded::byte_serialize(server.uri().as_bytes()).collect::<String>();
        let mut b = Request::builder().uri(format!("/oauth/callback?state={state}&iss={iss}&code=the-code"));
        if with_cookie {
            b = b.header(header::COOKIE, browser);
        }
        t.call(b.body(Body::empty()).unwrap()).await
    }

    #[tokio::test]
    async fn oauth_connect_like_and_discover() {
        let (server, seen) = network().await;
        let t = app(&server).await;
        let cookie = setup_user(&t).await;
        let c = Some(cookie.as_str());

        let meta = t.call(Request::builder().uri("/oauth/client-metadata.json").body(Body::empty()).unwrap()).await.json();
        assert_eq!(meta["token_endpoint_auth_method"], "private_key_jwt");
        let jwks = t.call(Request::builder().uri("/oauth/jwks.json").body(Body::empty()).unwrap()).await.json();
        assert_eq!(jwks["keys"][0]["crv"], "P-256");
        assert!(jwks["keys"][0].get("d").is_none(), "private key must not be published");

        assert!(t.web(Method::GET, "/api/atproto", c, None).await.json()["account"].is_null());

        let (_, bs) = connect(&t, &seen, &cookie, "@Alice.test").await;
        let page = callback(&t, &server, &bs, false).await;
        assert!(page.text.contains("Sign-in mismatch"), "{}", page.text);
        assert!(t.web(Method::GET, "/api/atproto", c, None).await.json()["account"].is_null());

        let (_, bs) = connect(&t, &seen, &cookie, "alice.test").await;
        let page = callback(&t, &server, &bs, true).await;
        assert!(page.text.contains("Bluesky connected"), "{}", page.text);
        let replay = callback(&t, &server, &bs, true).await;
        assert!(replay.text.contains("expired or was already used"), "{}", replay.text);
        let status = t.web(Method::GET, "/api/atproto", c, None).await.json();
        assert_eq!(status["account"]["did"], DID);
        assert_eq!(status["account"]["handle"], "alice.test");

        let r = t.web(Method::POST, "/api/feeds", c, Some(json!({"url": format!("{}/feed.xml", server.uri())}))).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.text);
        let entries = t.web(Method::GET, "/api/entries", c, None).await.json();
        let id_of = |title: &str| {
            entries["entries"].as_array().unwrap().iter().find(|e| e["title"] == title).unwrap()["id"].as_i64().unwrap()
        };
        let (doc_id, plain_id) = (id_of("Doc post"), id_of("Plain post"));

        let liked = t.web(Method::POST, &format!("/api/entries/{doc_id}/like"), c, None).await;
        assert_eq!(liked.status, StatusCode::OK, "{}", liked.text);
        assert_eq!(liked.json()["document_uri"], format!("at://{AUTHOR}/site.standard.document/doc1"));
        let liked_plain = t.web(Method::POST, &format!("/api/entries/{plain_id}/like"), c, None).await.json();
        assert!(liked_plain["document_uri"].is_null());
        {
            let records = seen.records.lock().unwrap();
            assert_eq!(records.len(), 3, "{records:?}");
            assert_eq!(records[0]["collection"], "app.skyreader.social.recommend");
            assert_eq!(records[0]["record"]["subject"], format!("{}/post1", server.uri()));
            assert_eq!(records[0]["record"]["document"], format!("at://{AUTHOR}/site.standard.document/doc1"));
            assert_eq!(records[1]["collection"], "site.standard.graph.recommend");
            assert_eq!(records[1]["record"]["document"], format!("at://{AUTHOR}/site.standard.document/doc1"));
            assert_eq!(records[2]["record"]["subject"], format!("{}/post2", server.uri()));
            assert!(records[2]["record"].get("document").is_none());
        }
        let listed = t.web(Method::GET, "/api/entries", c, None).await.json();
        assert!(listed["entries"].as_array().unwrap().iter().all(|e| e["like_uri"].is_string()));

        let r = t.web(Method::DELETE, &format!("/api/entries/{doc_id}/like"), c, None).await;
        assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", r.text);
        assert_eq!(seen.deleted.lock().unwrap().len(), 2);

        let pubs = t.web(Method::GET, "/api/atproto/publications", c, None).await;
        assert_eq!(pubs.status, StatusCode::OK, "{}", pubs.text);
        let pubs = pubs.json();
        let list = pubs["publications"].as_array().unwrap();
        assert_eq!(list.len(), 1, "{list:?}");
        assert_eq!(list[0]["handle"], "author.test");
        assert_eq!(list[0]["name"], "Author's Notes");

        let other = {
            let u = yesrss::auth::create_user(&t.state.db, "mallory", "mallorypassword").await.unwrap();
            let s = yesrss::auth::create_session(&t.state.db, u.id, yesrss::auth::SessionKind::Web).await.unwrap();
            format!("yesrss_session={s}")
        };
        let r = t.web(Method::POST, &format!("/api/entries/{plain_id}/like"), Some(&other), None).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND);

        sqlx::query("UPDATE atproto_accounts SET expires_at = 0").execute(&t.state.db).await.unwrap();
        let r = t.web(Method::POST, &format!("/api/entries/{doc_id}/like"), c, None).await;
        assert_eq!(r.status, StatusCode::OK, "{}", r.text);
        let token: String = sqlx::query_scalar("SELECT access_token FROM atproto_accounts").fetch_one(&t.state.db).await.unwrap();
        assert_eq!(token, "at-2", "expired token should have been refreshed");

        let r = t.web(Method::DELETE, "/api/atproto", c, None).await;
        assert_eq!(r.status, StatusCode::NO_CONTENT);
        assert!(t.web(Method::GET, "/api/atproto", c, None).await.json()["account"].is_null());
    }

    #[tokio::test]
    async fn disabled_without_public_url() {
        let t = TestApp::new().await;
        let c = setup_user(&t).await;
        assert_eq!(t.web(Method::GET, "/api/atproto", Some(&c), None).await.json()["enabled"], false);
        let r = t.call(Request::builder().uri("/oauth/client-metadata.json").body(Body::empty()).unwrap()).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND);
    }
}
