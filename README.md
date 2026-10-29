# yesrss

A small, safe, self-hosted feed reader in the spirit of Google Reader.

- **One static binary + one SQLite file.** No PHP, no Node, no external database, no build step for the UI.
- **Google Reader–style UI**: three-pane layout, folders with unread counts, list and expanded views,
  inline reading, keyboard-driven (`j`/`k`, `s`, `m`, `v`, `Shift+A`, `?` for the rest), dark mode, works on phones.
- **Google Reader API** compatible with FreshRSS clients: Reeder, NetNewsWire, ReadYou, FeedMe,
  FocusReader, Fluent Reader, Newsflash, … Served at both `/api/greader` and `/api/greader.php`,
  so apps pointed at FreshRSS only need the hostname changed.
- **OPML import/export**, folders and custom titles preserved.
- RSS 0.9–2.0, Atom, JSON Feed, podcasts (enclosures), YouTube channels (embedded player),
  feed autodiscovery from any web page, full-text search, favicons.

## Bluesky / atproto

Connect an atproto account (Settings → Bluesky) and yesrss can:

- **Like** any item (`l`, or the heart in the article footer). A like is an
  [`app.skyreader.social.recommend`](https://github.com/disnet/skyreader/tree/main/lexicons) record in your
  repo, so it is visible to Skyreader and anything else that reads that lexicon. When the article is a
  [standard.site](https://standard.site) document (its page has `<link rel="site.standard.document">`),
  yesrss also writes a `site.standard.graph.recommend`.
- **Find publications by people you follow**: standard.site publications from accounts you follow on
  Bluesky, one click to subscribe.

Sign-in is atproto OAuth (PAR + PKCE + DPoP, confidential client with an ES256 key generated on first use).
Your PDS's authorization server must be able to fetch this server's client metadata, so it needs a
**public HTTPS URL** (Pangolin, Caddy, Cloudflare Tunnel, Tailscale Funnel, …):

```sh
YESRSS_PUBLIC_URL=https://rss.example.com yesrss serve
```

Only `/oauth/client-metadata.json`, `/oauth/jwks.json` and `/oauth/callback` need to be reachable by the
outside world. yesrss requests the `atproto transition:generic` scope, because Bluesky's authorization
server does not yet advertise narrower scopes. Tokens are stored in the SQLite database, so protect it
like any credential store. Likes are public: every atproto record is.

Research notes and prior art: [`docs/research/atproto.md`](docs/research/atproto.md).

## Security posture

Feed content is hostile input, and a feed reader is a server that fetches URLs on request. yesrss treats
both seriously:

| Threat | Mitigation |
| --- | --- |
| XSS via feed HTML | Every entry is sanitized with [ammonia](https://github.com/rust-ammonia/ammonia) at ingest; strict `Content-Security-Policy` (`script-src 'self'`, no inline script or style) as a second wall. Iframes only from YouTube/Vimeo, sandboxed. |
| SSRF / LAN probing | Fetcher refuses loopback, private, link-local, CGNAT and other non-public addresses, checked **after DNS resolution and on every redirect**. Opt in with `--allow-private-networks`. |
| Resource exhaustion | Feed size cap (16 MiB default), favicon cap, timeouts, redirect limit, bounded fetch concurrency, exponential backoff, `Retry-After` honored. |
| Credential attacks | Argon2id password hashing, constant-time-ish login (dummy hash for unknown users), per-account rate limit, sessions stored as SHA-256 hashes. |
| CSRF | `SameSite=Strict` `HttpOnly` session cookie **and** a required custom header on all state-changing requests. The Google Reader API uses bearer tokens, not cookies. |
| Malicious favicons | Only PNG/ICO/GIF/JPEG/WebP accepted, detected by magic bytes (never SVG), served with `CSP: sandbox`. |
| Clickjacking, sniffing, referrer leaks | `frame-ancestors 'none'`, `X-Frame-Options: DENY`, `nosniff`, `Referrer-Policy: no-referrer`. |

There is no `unsafe` code in yesrss itself.

## Quick start

### Docker / Podman

Multi-arch images (amd64, arm64) are published to `ghcr.io/derekparks/yesrss`: `edge` tracks `main`,
`latest` and `X.Y.Z` track release tags.

```sh
docker run -d --name yesrss -p 8080:8080 -v yesrss-data:/data ghcr.io/derekparks/yesrss:edge
# or, with the hardened settings in compose.yaml:
docker compose up -d
```

The first visit to http://localhost:8080 creates the owner account.

### Binary

```sh
cargo build --release
./target/release/yesrss serve --listen 0.0.0.0:8080 --db /var/lib/yesrss/yesrss.db
```

A hardened systemd unit is in [`contrib/yesrss.service`](contrib/yesrss.service).

### Behind HTTPS (Caddy, Traefik, `tailscale serve`, …)

Run yesrss on localhost and set `YESRSS_SECURE_COOKIES=true`. For Tailscale:

```sh
yesrss serve --listen 127.0.0.1:8080 --secure-cookies
tailscale serve --bg 8080
```

## Migrating from FreshRSS

1. In FreshRSS: **Subscription management → Import / export → Export** (OPML).
2. In yesrss: **Settings → Import / export → Choose OPML file…**, or from the CLI:
   `yesrss import-opml <username> freshrss.opml`
3. In your mobile app, change the server from `https://fresh.example/api/greader.php` to
   `https://yesrss.example/api/greader.php` (or `/api/greader`) and sign in with your yesrss credentials.

Read/starred history is not carried over by OPML. Everything currently in your feeds arrives as unread;
use **Mark all read** once after importing.

## Configuration

Every flag also reads an environment variable.

| Flag | Env | Default | |
| --- | --- | --- | --- |
| `--db` | `YESRSS_DB` | `yesrss.db` | SQLite database path |
| `--listen` | `YESRSS_LISTEN` | `127.0.0.1:8080` | Bind address |
| `--secure-cookies` | `YESRSS_SECURE_COOKIES` | off | Set `Secure` on cookies (use with HTTPS) |
| `--allow-private-networks` | `YESRSS_ALLOW_PRIVATE_NETWORKS` | off | Permit feeds on LAN/loopback/tailnet addresses |
| `--refresh-minutes` | `YESRSS_REFRESH_MINUTES` | `30` | Refresh interval for healthy feeds |
| `--retention-days` | `YESRSS_RETENTION_DAYS` | `90` | Keep read, unstarred entries this long once they leave the feed |
| `--fetch-concurrency` | `YESRSS_FETCH_CONCURRENCY` | `8` | Parallel fetches |
| `--public-url` | `YESRSS_PUBLIC_URL` | unset | Public HTTPS URL; enables Bluesky / atproto features |
| `--max-feed-bytes` | `YESRSS_MAX_FEED_BYTES` | `16777216` | Largest accepted feed |
| `--link-rules` | `YESRSS_LINK_RULES` | unset | TOML file of extra [link rules](#link-rules) |

Logging uses `RUST_LOG` (default `info`).

### Link rules

Some feed services wrap every link in a click-tracking redirect, and not all of those redirects work:
FeedBlitz sends some of its own wrapped links to a "fake link" page. yesrss rewrites links when it
stores an item, using [`src/link_rules.toml`](src/link_rules.toml):

- `[[unwrap]]` replaces a wrapper link with the URL it wraps (FeedBlitz, Outlook Safe Links).
- `[[drop]]` removes links and images such as share buttons and tracking pixels.

Items from FeedBurner-style feeds also link to `feedburner:origLink`, the article itself, rather than
the redirect. To add rules without rebuilding, put them in a file of the same format and point
`YESRSS_LINK_RULES` at it; they are added to the built-in set. When the rules change, the next start
re-applies them to items already stored.

## User management

The first account is created through the web UI on first visit. Everything else is CLI:

```sh
yesrss user add alice            # prompts, or reads YESRSS_PASSWORD / stdin
yesrss user passwd alice         # also signs out every session
yesrss user list
yesrss user delete alice
```

With Docker: `docker compose exec yesrss yesrss user add alice`.

## Backups

The database runs in WAL mode. Take a consistent snapshot while it is running with:

```sh
sqlite3 /var/lib/yesrss/yesrss.db ".backup yesrss-backup.db"
```

## Development

```sh
cargo test                       # unit + end-to-end tests (feed server mocked with wiremock)
cargo run -- serve --allow-private-networks
```

The UI is plain ES modules in `static/`, embedded into the binary at build time (read from disk in
debug builds, so reload the browser to see changes).

| Path | |
| --- | --- |
| `src/fetch.rs` | HTTP client, SSRF guard, conditional GET |
| `src/parse.rs`, `src/sanitize.rs` | Feed normalization and HTML sanitizing |
| `src/links.rs`, `src/link_rules.toml` | Link unwrapping and stripping rules |
| `src/feeds.rs`, `src/scheduler.rs` | Subscribe, discover, refresh, favicons, retention |
| `src/entries.rs` | Entry queries shared by both APIs |
| `src/api.rs` | JSON API for the web UI |
| `src/greader.rs` | Google Reader API |
| `src/atproto/` | atproto OAuth client, identity resolution, likes, publication discovery |
| `static/` | Web UI |
