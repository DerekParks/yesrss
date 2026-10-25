# atproto social features: prior art and schemas

Researched 2026-10-25. Schemas below were read from the authorities' own `com.atproto.lexicon.schema`
records (resolved via `_lexicon.<authority>` DNS TXT) or from the source repo, not from summaries.

## TL;DR

Most of this has already been designed. **Skyreader** ([disnet/skyreader](https://github.com/disnet/skyreader),
AGPL-3.0, active) is an atproto-native RSS reader whose lexicons model URL likes, synced subscriptions,
and subscriptions to atproto authors. **standard.site** defines the long-form publishing records. Reusing
both makes yesrss interoperate with existing apps on day one instead of inventing `app.yesrss.*`.

| Feature | Write / read this | Owner |
| --- | --- | --- |
| Like any RSS item | `app.skyreader.social.recommend` | Skyreader |
| Like a standard.site article | also `site.standard.graph.recommend` | standard.site |
| Star → public bookmark (optional) | `community.lexicon.bookmarks.bookmark` | lexicon.community |
| Sync subscriptions to your PDS (optional) | `app.skyreader.feed.subscription` | Skyreader |
| See followed people's publications | read `site.standard.publication` / `site.standard.document` | standard.site |
| Follow a publication | `site.standard.graph.subscription` | standard.site |

## Lexicons

### `app.skyreader.social.recommend` (key: tid)
"A public recommendation of an article. The subject is the article's URL, so any web page can be
recommended; when the article is itself an atproto document, `document` points at that record too."

| field | type | required |
| --- | --- | --- |
| `subject` | uri (article URL) | yes |
| `document` | at-uri (e.g. a `site.standard.document`) | no |
| `createdAt` | datetime | yes |

This is the right shape for a yesrss like: works for every RSS item, upgrades to a record link when the
item is a standard.site document.

### `site.standard.graph.recommend` (key: tid)
"Record declaring a recommendation of a document." Requires `document` (at-uri) and `createdAt`.
**Cannot like arbitrary URLs.** Only usable when the entry resolves to a `site.standard.document`.

### `site.standard.graph.subscription` (key: tid)
Subscription to a publication. Requires `publication` (at-uri); optional `createdAt`.

### `site.standard.document` (key: tid)
Required: `site` (at-uri of a `site.standard.publication`, or an https URL for loose documents),
`title`, `publishedAt`. Useful optional: `path` (canonical URL = publication `url` + `path`),
`description`, `textContent` (plaintext), `content` (open union), `coverImage`, `tags`, `updatedAt`,
`bskyPostRef` (strongRef to a Bluesky post for comments), `contributors`, `labels`, `links`.

### `site.standard.publication`
Required `url` and `name`; optional `description`, `icon`, `theme`, `labels`, `preferences`.

**Verification:** the site serves `/.well-known/site.standard.publication` returning the publication
AT-URI, and each article page carries `<link rel="site.standard.document" href="at://…">`. That `<link>`
is how yesrss can tell an RSS item *is* a standard.site document. Fetch the article page, or match the
item URL against indexed `publication.url + path`.

### `app.skyreader.feed.subscription` (key: tid)
`feedUrl`, `title`, `siteUrl`, `category` (≈ yesrss folder), `tags`, `customTitle`, `customIconUrl`,
`createdAt` (required), `updatedAt`, `source` ('opml', 'manual', 'leaflet', …), `externalRef` (at-uri),
`sourceType` ('rss' | 'atproto.shares' | 'atproto.documents' | 'atproto.collection'), `subjectDid`,
`collectionNsid`. `sourceType: atproto.documents` + `subjectDid` already models "subscribe to an atproto
author's standard.site documents".

Skyreader also defines `app.skyreader.social.follow` (in-app follow by DID) and
`app.skyreader.reading.readAlong`.

### `community.lexicon.bookmarks.bookmark`
Shared bookmark record (URL, tags, createdAt) used by kipclip, bluemarks, and the Obsidian Atmosphere
plugin. Schema fields not re-verified here. Check the lexicon.community repo before implementing.

## Auth

- **OAuth is the supported path.** It requires PKCE, PAR, and DPoP with server nonces. Confidential
  (server-side) clients authenticate with a signed JWT ([spec](https://atproto.com/specs/auth),
  [guide](https://atproto.com/guides/oauth)). Rust options: `jacquard-oauth` 0.12.2 (updated 2026-09,
  most downloads) and `atrium-oauth` 0.1.7 (updated 2026-03). Neither has been evaluated yet.
- **Homelab constraint.** The `client_id` is an https URL to client metadata that the user's PDS
  authorization server fetches. A yesrss reachable only on a tailnet can't be fetched by
  `bsky.network`, so OAuth needs a public hostname (Tailscale Funnel, a tunnel, or a public metadata host
  if the spec's origin rules allow it; unverified). App passwords ("login with a token") work with no
  inbound reachability.
- Either way, an atproto identity should be a **connected account** on an existing yesrss user, not a
  replacement for local login. The Google Reader API still needs local credentials.

## Seeing what people you follow publish

1. Read the user's follows (`app.bsky.graph.follow` records in their repo, or the public AppView's
   `app.bsky.graph.getFollows`).
2. For each followed DID, `com.atproto.repo.listRecords` on `site.standard.publication`.
3. Offer each as a subscription: use the publication's RSS feed if it has one, otherwise render
   `site.standard.document` records directly (`title`, `textContent`/`content`, canonical URL).

This is polling, which is fine for one user. No indexer is needed yet.

## Later: a global "for you" AppView

Prior art: Steve's [Indexing Standard.site](https://atproto.com/blog/indexing-standard-site) post
(April 2026) and [docs.surf](https://docs.surf), which indexes every `site.standard.document`.

- **Jetstream** filters server-side to the listed collections. It has no backfill but is cheap.
  **Tap** filters the full firehose locally and backfills, but the post reports high egress costs and
  ISP throttling on a home connection.
- Resolve asynchronously through a queue (up to ~4 requests per document), and re-verify failed
  `.well-known` / `<link>` checks on a cron. Publishers create records before their site deploys, so
  immediate verification gives false negatives.
- A "for you" feed could aggregate `app.skyreader.social.recommend` + `site.standard.graph.recommend`
  from your follows and serve it as plain RSS/Atom. Any reader could then subscribe, yesrss included.

## Open questions

1. Is this instance publicly reachable (needed for OAuth), or is it tailnet-only (app passwords)?
2. Should likes be public by default? Every record on a PDS is public.
3. Sync subscriptions to the PDS as `app.skyreader.feed.subscription`? That gives Skyreader
   interop, but your subscription list becomes public.
4. Coordinate with the Skyreader author before depending on `app.skyreader.*`. The lexicons are
   theirs to evolve.

## Sources

- [disnet/skyreader](https://github.com/disnet/skyreader) — `lexicons/app/skyreader/**` (read 2026-10-25)
- [Skyreader announcement](https://www.disnetdev.com/blog/2026-01-20-skyreader-a-rss-reader-on-the-at-protocol/)
- [standard.site](https://standard.site), and lexicon records from `did:plc:re3ebnp5v7ffagz6rb6xfei4`
  (resolved via `_lexicon.standard.site`)
- [Indexing Standard Site](https://atproto.com/blog/indexing-standard-site)
- [atproto auth spec](https://atproto.com/specs/auth), [OAuth guide](https://atproto.com/guides/oauth)
- [kipclip-cli](https://tangled.org/tijs.org/kipclip-cli), [bluemarks](https://tangled.org/hyl.st/bluemarks),
  [Obsidian Atmosphere](https://bmannconsulting.com/notes/obsidian-atmosphere/)
- crates.io: `jacquard-oauth`, `atrium-oauth`
