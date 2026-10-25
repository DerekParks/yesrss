use feed_rs::model::{Entry, Feed, Link, Text};
use sha2::{Digest, Sha256};
use url::Url;

use crate::sanitize::{clean_html, clean_title, escape_html, html_to_text, text_to_html, truncate_chars};

const SNIPPET_CHARS: usize = 300;
const MAX_GUID_LEN: usize = 2048;

#[derive(Debug)]
pub struct ParsedFeed {
    pub title: String,
    pub site_url: Option<String>,
    pub entries: Vec<NewEntry>,
}

#[derive(Debug, Clone)]
pub struct NewEntry {
    pub guid: String,
    pub url: Option<String>,
    pub title: String,
    pub author: Option<String>,
    pub content: String,
    pub snippet: String,
    pub search_text: String,
    pub enclosure_url: Option<String>,
    pub enclosure_type: Option<String>,
    pub published_at: i64,
    pub content_hash: String,
}

pub fn parse_feed(body: &[u8], source: &Url, now: i64) -> anyhow::Result<ParsedFeed> {
    let feed = feed_rs::parser::Builder::new()
        .base_uri(Some(source.as_str()))
        .build()
        .parse(body)?;
    Ok(convert_feed(feed, source, now))
}

fn convert_feed(feed: Feed, source: &Url, now: i64) -> ParsedFeed {
    let site_url = pick_link(&feed.links)
        .and_then(|href| source.join(href).ok())
        .filter(|u| matches!(u.scheme(), "http" | "https") && u != source)
        .map(String::from);
    let title = feed
        .title
        .as_ref()
        .map(|t| clean_title(&t.content))
        .filter(|t| !t.is_empty())
        .or_else(|| source.host_str().map(str::to_owned))
        .unwrap_or_else(|| source.to_string());
    let entries = feed
        .entries
        .into_iter()
        .map(|e| convert_entry(e, source, now))
        .collect();
    ParsedFeed { title, site_url, entries }
}

fn pick_link(links: &[Link]) -> Option<&str> {
    links
        .iter()
        .find(|l| {
            matches!(l.rel.as_deref(), None | Some("alternate"))
                && l.media_type.as_deref().is_none_or(|t| t.contains("html"))
        })
        .or_else(|| links.iter().find(|l| l.rel.as_deref() != Some("self")))
        .map(|l| l.href.as_str())
}

fn text_html(t: &Text) -> String {
    if t.content_type.to_string().contains("html") {
        t.content.clone()
    } else {
        text_to_html(&t.content)
    }
}

fn youtube_id(url: &Url) -> Option<String> {
    let host = url.host_str()?.trim_start_matches("www.").trim_start_matches("m.");
    match host {
        "youtube.com" if url.path() == "/watch" => url
            .query_pairs()
            .find(|(k, _)| k == "v")
            .map(|(_, v)| v.into_owned()),
        "youtube.com" => url.path().strip_prefix("/shorts/").map(str::to_owned),
        "youtu.be" => Some(url.path().trim_start_matches('/').to_owned()),
        _ => None,
    }
    .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
}

fn convert_entry(e: Entry, source: &Url, now: i64) -> NewEntry {
    let url = pick_link(&e.links)
        .and_then(|href| source.join(href).ok())
        .filter(|u| matches!(u.scheme(), "http" | "https"));
    let base = url.as_ref().unwrap_or(source);

    let media = e.media.first();
    let mut raw_html = e
        .content
        .as_ref()
        .and_then(|c| {
            let body = c.body.as_ref()?;
            Some(if c.content_type.to_string().contains("html") {
                body.clone()
            } else {
                text_to_html(body)
            })
        })
        .or_else(|| e.summary.as_ref().map(text_html))
        .or_else(|| media.and_then(|m| m.description.as_ref()).map(text_html))
        .unwrap_or_default();

    if let Some(id) = url.as_ref().and_then(youtube_id) {
        raw_html = format!(
            r#"<iframe src="https://www.youtube-nocookie.com/embed/{id}" width="640" height="360" allowfullscreen title="YouTube video"></iframe>{raw_html}"#
        );
    } else if let Some(thumb) = media.and_then(|m| m.thumbnails.first())
        && !raw_html.contains("<img")
    {
        raw_html = format!(r#"<p><img src="{}" alt=""></p>{raw_html}"#, escape_html(&thumb.image.uri));
    }

    let content = clean_html(&raw_html, Some(base));
    let text = html_to_text(&content);
    let title = e
        .title
        .as_ref()
        .map(|t| clean_title(&t.content))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| truncate_chars(&text, 80));
    let author = e
        .authors
        .first()
        .and_then(|p| p.name.as_deref())
        .map(clean_title)
        .filter(|a| !a.is_empty());

    let (enclosure_url, enclosure_type) = e
        .links
        .iter()
        .find(|l| l.rel.as_deref() == Some("enclosure"))
        .and_then(|l| Some((source.join(&l.href).ok()?, l.media_type.clone())))
        .or_else(|| {
            e.media.iter().flat_map(|m| &m.content).find_map(|c| {
                let ty = c.content_type.as_ref()?.to_string();
                (ty.starts_with("audio/") || ty.starts_with("video/"))
                    .then(|| Some((c.url.clone()?, Some(ty))))
                    .flatten()
            })
        })
        .filter(|(u, _)| matches!(u.scheme(), "http" | "https"))
        .map(|(u, t)| (Some(String::from(u)), t))
        .unwrap_or((None, None));

    let published_at = e
        .published
        .or(e.updated)
        .map(|d| d.timestamp())
        .filter(|&t| t > 0)
        .map_or(now, |t| t.min(now));

    let mut guid = if e.id.is_empty() {
        url.as_ref().map(Url::to_string).unwrap_or_else(|| title.clone())
    } else {
        e.id
    };
    if guid.len() > MAX_GUID_LEN {
        guid = hex::encode(Sha256::digest(guid.as_bytes()));
    }

    let url = url.map(String::from);
    let content_hash = {
        let mut h = Sha256::new();
        for part in [title.as_str(), url.as_deref().unwrap_or(""), content.as_str()] {
            h.update(part.as_bytes());
            h.update([0]);
        }
        hex::encode(h.finalize())
    };

    NewEntry {
        guid,
        url,
        snippet: truncate_chars(&text, SNIPPET_CHARS),
        search_text: text,
        title,
        author,
        content,
        enclosure_url,
        enclosure_type,
        published_at,
        content_hash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    fn src() -> Url {
        Url::parse("https://example.com/feed.xml").unwrap()
    }

    #[test]
    fn parses_rss() {
        let xml = br#"<?xml version="1.0"?>
<rss version="2.0"><channel><title>Example &amp; Co</title><link>https://example.com/</link>
<item><title>First</title><link>https://example.com/1</link><guid>id-1</guid>
<description>&lt;p&gt;Hello &lt;script&gt;x()&lt;/script&gt;&lt;img src="/a.png"&gt;&lt;/p&gt;</description>
<pubDate>Tue, 10 Jun 2025 04:00:00 GMT</pubDate>
<enclosure url="https://example.com/ep1.mp3" type="audio/mpeg" length="1"/></item>
<item><title>Future</title><guid>id-2</guid><pubDate>Tue, 10 Jun 2099 04:00:00 GMT</pubDate></item>
</channel></rss>"#;
        let f = parse_feed(xml, &src(), NOW).unwrap();
        assert_eq!(f.title, "Example & Co");
        assert_eq!(f.site_url.as_deref(), Some("https://example.com/"));
        assert_eq!(f.entries.len(), 2);
        let e = &f.entries[0];
        assert_eq!(e.guid, "id-1");
        assert_eq!(e.url.as_deref(), Some("https://example.com/1"));
        assert!(!e.content.contains("script"));
        assert!(e.content.contains("https://example.com/a.png"));
        assert_eq!(e.snippet, "Hello");
        assert_eq!(e.enclosure_url.as_deref(), Some("https://example.com/ep1.mp3"));
        assert_eq!(e.enclosure_type.as_deref(), Some("audio/mpeg"));
        assert_eq!(e.published_at, 1_749_528_000);
        assert_eq!(f.entries[1].published_at, NOW);
    }

    #[test]
    fn parses_atom_and_embeds_youtube() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xmlns:media="http://search.yahoo.com/mrss/">
<title>Channel</title><link rel="alternate" href="https://www.youtube.com/channel/x"/>
<entry><id>yt:video:abc_123</id><title>Video</title>
<link rel="alternate" href="https://www.youtube.com/watch?v=abc_123"/>
<author><name>Someone</name></author><published>2025-01-01T00:00:00Z</published>
<media:group><media:title>Video</media:title><media:content url="https://www.youtube.com/v/abc_123" type="application/x-shockwave-flash" width="640" height="390"/><media:thumbnail url="https://i1.ytimg.com/vi/abc_123/hqdefault.jpg" width="480" height="360"/><media:description>Line one</media:description></media:group></entry></feed>"#;
        let f = parse_feed(xml, &src(), NOW).unwrap();
        let e = &f.entries[0];
        assert_eq!(e.author.as_deref(), Some("Someone"));
        assert!(e.content.contains("youtube-nocookie.com/embed/abc_123"), "{}", e.content);
        assert!(e.content.contains("Line one"), "{}", e.content);
    }

    #[test]
    fn parses_json_feed() {
        let json = br#"{"version":"https://jsonfeed.org/version/1.1","title":"JF",
"items":[{"id":"1","content_text":"plain <text>","url":"https://example.com/p"}]}"#;
        let f = parse_feed(json, &src(), NOW).unwrap();
        assert_eq!(f.title, "JF");
        assert_eq!(f.entries[0].content, "<p>plain &lt;text&gt;</p>");
        assert_eq!(f.entries[0].title, "plain <text>");
    }

    #[test]
    fn rejects_html() {
        assert!(parse_feed(b"<!doctype html><html><body>hi</body></html>", &src(), NOW).is_err());
    }
}
