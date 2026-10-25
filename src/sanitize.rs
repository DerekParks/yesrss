use std::borrow::Cow;
use std::collections::HashSet;

use ammonia::{Builder, UrlRelative};
use url::Url;

const EMBED_HOSTS: &[&str] = &[
    "www.youtube.com",
    "youtube.com",
    "www.youtube-nocookie.com",
    "youtube-nocookie.com",
    "player.vimeo.com",
];

pub fn clean_html(html: &str, base: Option<&Url>) -> String {
    let base = base.cloned();
    let mut b = Builder::default();
    b.add_tags(["iframe", "video", "audio", "source", "picture"])
        .add_tag_attributes("img", ["srcset", "title"])
        .add_tag_attributes("video", ["src", "poster", "width", "height", "preload"])
        .add_tag_attributes("audio", ["src", "preload"])
        .add_tag_attributes("source", ["src", "srcset", "type", "media"])
        .add_tag_attributes("iframe", ["src", "width", "height", "title", "allowfullscreen"])
        .set_tag_attribute_value("img", "loading", "lazy")
        .set_tag_attribute_value("iframe", "loading", "lazy")
        .set_tag_attribute_value(
            "iframe",
            "sandbox",
            "allow-scripts allow-same-origin allow-popups allow-presentation",
        )
        .set_tag_attribute_value("iframe", "referrerpolicy", "strict-origin-when-cross-origin")
        .set_tag_attribute_value("video", "controls", "")
        .set_tag_attribute_value("audio", "controls", "")
        .set_tag_attribute_value("a", "target", "_blank")
        .link_rel(Some("noopener noreferrer"))
        .url_schemes(HashSet::from(["http", "https", "mailto", "magnet"]))
        .strip_comments(true);
    match &base {
        Some(u) => b.url_relative(UrlRelative::RewriteWithBase(u.clone())),
        None => b.url_relative(UrlRelative::Deny),
    };
    b.attribute_filter(move |element, attribute, value| match (element, attribute) {
        ("iframe", "src") => embed_src(value, base.as_ref()).map(Cow::Owned),
        (_, "srcset") => clean_srcset(value, base.as_ref()).map(Cow::Owned),
        _ => Some(Cow::Borrowed(value)),
    });
    b.clean(html).to_string()
}

fn absolute_http(raw: &str, base: Option<&Url>) -> Option<Url> {
    let url = match base {
        Some(b) => b.join(raw.trim()).ok()?,
        None => Url::parse(raw.trim()).ok()?,
    };
    matches!(url.scheme(), "http" | "https").then_some(url)
}

fn embed_src(value: &str, base: Option<&Url>) -> Option<String> {
    let mut url = absolute_http(value, base)?;
    let host = url.host_str()?.to_ascii_lowercase();
    if !EMBED_HOSTS.contains(&host.as_str()) {
        return None;
    }
    url.set_scheme("https").ok()?;
    Some(url.into())
}

fn clean_srcset(value: &str, base: Option<&Url>) -> Option<String> {
    let parts: Vec<String> = value
        .split(',')
        .filter_map(|candidate| {
            let mut it = candidate.split_whitespace();
            let url = absolute_http(it.next()?, base)?;
            let descriptor: Vec<&str> = it.collect();
            Some(if descriptor.is_empty() {
                url.to_string()
            } else {
                format!("{url} {}", descriptor.join(" "))
            })
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

pub fn html_to_text(html: &str) -> String {
    let frag = scraper::Html::parse_fragment(html);
    let text: String = frag.root_element().text().collect::<Vec<_>>().join(" ");
    collapse_whitespace(&text)
}

pub fn collapse_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn clean_title(raw: &str) -> String {
    if raw.contains('<') || raw.contains('&') {
        html_to_text(raw)
    } else {
        collapse_whitespace(raw)
    }
}

pub fn truncate_chars(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => format!("{}…", s[..i].trim_end()),
        None => s.to_owned(),
    }
}

pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

pub fn text_to_html(text: &str) -> String {
    text.split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| format!("<p>{}</p>", escape_html(p).replace('\n', "<br>")))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://blog.example.com/posts/1").unwrap()
    }

    #[test]
    fn strips_scripts_and_handlers() {
        let out = clean_html(
            r#"<p onclick="evil()">hi<script>alert(1)</script></p><img src=x onerror=alert(1)><a href="javascript:alert(1)">x</a><style>body{}</style>"#,
            Some(&base()),
        );
        assert!(!out.contains("script"), "{out}");
        assert!(!out.contains("onclick"), "{out}");
        assert!(!out.contains("onerror"), "{out}");
        assert!(!out.contains("javascript"), "{out}");
        assert!(!out.contains("<style"), "{out}");
    }

    #[test]
    fn rewrites_relative_urls_and_hardens_links() {
        let out = clean_html(r#"<a href="/about">a</a><img src="pic.png">"#, Some(&base()));
        assert!(out.contains(r#"href="https://blog.example.com/about""#), "{out}");
        assert!(out.contains(r#"src="https://blog.example.com/posts/pic.png""#), "{out}");
        assert!(out.contains(r#"target="_blank""#), "{out}");
        assert!(out.contains("noopener"), "{out}");
        assert!(out.contains(r#"loading="lazy""#), "{out}");
    }

    #[test]
    fn allows_only_known_iframes() {
        let yt = clean_html(r#"<iframe src="https://www.youtube.com/embed/abc"></iframe>"#, None);
        assert!(yt.contains("youtube.com/embed/abc"), "{yt}");
        assert!(yt.contains("sandbox="), "{yt}");
        let evil = clean_html(r#"<iframe src="https://evil.example/phish"></iframe>"#, None);
        assert!(!evil.contains("evil.example"), "{evil}");
    }

    #[test]
    fn cleans_srcset() {
        let out = clean_html(
            r#"<img src="a.png" srcset="a.png 1x, javascript:alert(1) 2x, //cdn.example.com/b.png 3x">"#,
            Some(&base()),
        );
        assert!(out.contains("https://blog.example.com/posts/a.png 1x"), "{out}");
        assert!(out.contains("https://cdn.example.com/b.png 3x"), "{out}");
        assert!(!out.contains("javascript"), "{out}");
    }

    #[test]
    fn titles_and_text() {
        assert_eq!(clean_title("Tom &amp; Jerry <b>live</b>"), "Tom & Jerry live");
        assert_eq!(clean_title("  plain\n title "), "plain title");
        assert_eq!(truncate_chars("héllo world", 5), "héllo…");
        assert_eq!(text_to_html("a<b\nc\n\nd"), "<p>a&lt;b<br>c</p><p>d</p>");
    }
}
