use std::borrow::Cow;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, ensure};
use percent_encoding::percent_decode_str;
use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use url::Url;

use crate::db::begin_write;
use crate::sanitize::clean_html;

const BUILTIN: &str = include_str!("link_rules.toml");
const MAX_UNWRAP_DEPTH: usize = 8;
const REAPPLY_BATCH: i64 = 500;
const FINGERPRINT_SETTING: &str = "link_rules_fingerprint";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleFile {
    #[serde(default)]
    unwrap: Vec<UnwrapSpec>,
    #[serde(default)]
    drop: Vec<DropSpec>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnwrapSpec {
    name: String,
    pattern: String,
    #[serde(default)]
    decode: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DropSpec {
    name: String,
    pattern: String,
}

struct Unwrap {
    re: Regex,
    decode: bool,
}

impl Unwrap {
    fn apply(&self, url: &str) -> Option<Url> {
        let inner = self.re.captures(url)?.name("url")?.as_str();
        let inner = if self.decode { percent_decode_str(inner).decode_utf8().ok()? } else { Cow::Borrowed(inner) };
        Url::parse(inner.trim()).ok().filter(|u| matches!(u.scheme(), "http" | "https"))
    }
}

struct Rules {
    unwrap: Vec<Unwrap>,
    drop: Vec<Regex>,
    fingerprint: String,
}

/// The `[[unwrap]]` and `[[drop]]` rules from `link_rules.toml`, plus any from `YESRSS_LINK_RULES`.
#[derive(Clone)]
pub struct LinkRules(Arc<Rules>);

impl Default for LinkRules {
    fn default() -> Self {
        Self::from_sources(&[("built-in link rules", BUILTIN)]).expect("built-in link rules are valid")
    }
}

impl LinkRules {
    pub fn load(extra: Option<&Path>) -> anyhow::Result<Self> {
        let Some(path) = extra else { return Ok(Self::default()) };
        let text = std::fs::read_to_string(path).with_context(|| format!("reading link rules {}", path.display()))?;
        Self::from_sources(&[("built-in link rules", BUILTIN), (&path.display().to_string(), &text)])
    }

    fn from_sources(sources: &[(&str, &str)]) -> anyhow::Result<Self> {
        let mut hash = Sha256::new();
        let (mut unwrap, mut drop) = (Vec::new(), Vec::new());
        for (origin, text) in sources {
            hash.update(text.as_bytes());
            hash.update([0]);
            let file: RuleFile = toml::from_str(text).with_context(|| format!("parsing {origin}"))?;
            for u in file.unwrap {
                let re = Regex::new(&u.pattern).with_context(|| format!("{origin}: unwrap rule {:?}", u.name))?;
                ensure!(
                    re.capture_names().any(|n| n == Some("url")),
                    "{origin}: unwrap rule {:?} has no (?<url>…) group",
                    u.name
                );
                unwrap.push(Unwrap { re, decode: u.decode });
            }
            for d in file.drop {
                drop.push(Regex::new(&d.pattern).with_context(|| format!("{origin}: drop rule {:?}", d.name))?);
            }
        }
        Ok(Self(Arc::new(Rules { unwrap, drop, fingerprint: hex::encode(hash.finalize()) })))
    }

    pub fn fingerprint(&self) -> &str {
        &self.0.fingerprint
    }

    /// The URL `raw` leads to once every wrapper is peeled off; `None` when no rule matches.
    pub fn unwrap_url(&self, raw: &str) -> Option<Url> {
        let mut found: Option<Url> = None;
        for _ in 0..MAX_UNWRAP_DEPTH {
            let current = found.as_ref().map_or(raw.trim(), Url::as_str);
            match self.0.unwrap.iter().find_map(|u| u.apply(current)) {
                Some(next) => found = Some(next),
                None => break,
            }
        }
        found
    }

    fn drops(&self, url: &str) -> bool {
        self.0.drop.iter().any(|re| re.is_match(url.trim()))
    }

    /// Removes the links and images that `[[drop]]` rules match.
    pub fn strip<'a>(&self, html: &'a str) -> Cow<'a, str> {
        if self.0.drop.is_empty() {
            return Cow::Borrowed(html);
        }
        let mut doc = scraper::Html::parse_fragment(html);
        let doomed: Vec<_> = doc
            .tree
            .nodes()
            .filter(|node| {
                node.value().as_element().is_some_and(|el| {
                    let attr = match el.name() {
                        "a" => "href",
                        "img" => "src",
                        _ => return false,
                    };
                    el.attr(attr).is_some_and(|url| self.drops(url))
                })
            })
            .map(|node| node.id())
            .collect();
        if doomed.is_empty() {
            return Cow::Borrowed(html);
        }
        for id in doomed {
            if let Some(mut node) = doc.tree.get_mut(id) {
                node.detach();
            }
        }
        Cow::Owned(doc.root_element().inner_html())
    }
}

/// Re-applies `rules` to every stored entry unless they already were; returns how many changed.
pub async fn reapply_if_changed(db: &SqlitePool, rules: &LinkRules) -> anyhow::Result<Option<usize>> {
    let applied: Option<String> = sqlx::query_scalar("SELECT value FROM app_settings WHERE name = ?")
        .bind(FINGERPRINT_SETTING)
        .fetch_optional(db)
        .await?;
    if applied.as_deref() == Some(rules.fingerprint()) {
        return Ok(None);
    }
    let changed = reapply(db, rules).await?;
    sqlx::query("INSERT INTO app_settings (name, value) VALUES (?, ?) ON CONFLICT (name) DO UPDATE SET value = excluded.value")
        .bind(FINGERPRINT_SETTING)
        .bind(rules.fingerprint())
        .execute(db)
        .await?;
    Ok(Some(changed))
}

async fn reapply(db: &SqlitePool, rules: &LinkRules) -> anyhow::Result<usize> {
    let (mut after, mut changed) = (0_i64, 0);
    loop {
        let mut tx = begin_write(db).await?;
        let rows: Vec<(i64, Option<String>, String)> =
            sqlx::query_as("SELECT id, url, content FROM entries WHERE id > ? ORDER BY id LIMIT ?")
                .bind(after)
                .bind(REAPPLY_BATCH)
                .fetch_all(&mut *tx)
                .await?;
        let Some(&(last, _, _)) = rows.last() else { break };
        for (id, url, content) in rows {
            let base = url.as_deref().and_then(|u| Url::parse(u).ok());
            let new_url = base.as_ref().and_then(|u| rules.unwrap_url(u.as_str())).or(base);
            let new_content = clean_html(&content, new_url.as_ref(), rules);
            let new_url = new_url.map(String::from);
            if new_url == url && new_content == content {
                continue;
            }
            sqlx::query("UPDATE entries SET url = ?, content = ? WHERE id = ?")
                .bind(new_url)
                .bind(new_content)
                .bind(id)
                .execute(&mut *tx)
                .await?;
            changed += 1;
        }
        tx.commit().await?;
        after = last;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARTICLE: &str = "https://saez-zucman-audit-replication.vercel.app/";

    fn unwrapped(raw: &str) -> Option<String> {
        LinkRules::default().unwrap_url(raw).map(String::from)
    }

    #[test]
    fn unwraps_feedblitz_click_tracking() {
        let wrapped = format!("https://feeds.feedblitz.com/~/t/0/0/marginalrevolution/~{ARTICLE}");
        assert_eq!(unwrapped(&wrapped).as_deref(), Some(ARTICLE));
        let with_query = "https://feeds.feedblitz.com/~/t/0/0/marginalrevolution/~https://www.amazon.com/dp/1?tag=x-20#top";
        assert_eq!(unwrapped(with_query).as_deref(), Some("https://www.amazon.com/dp/1?tag=x-20#top"));
    }

    #[test]
    fn leaves_other_links_alone() {
        assert_eq!(unwrapped("https://marginalrevolution.com/marginalrevolution/2026/10/x.html"), None);
        assert_eq!(unwrapped("https://feeds.feedblitz.com/~/972063845/0/marginalrevolution~Title.html"), None);
        assert_eq!(unwrapped("https://feeds.feedblitz.com/~/t/0/0/mr/~javascript:alert(1)"), None);
    }

    #[test]
    fn unwraps_nested_and_encoded_wrappers() {
        let wrapped = "https://feeds.feedblitz.com/~/t/0/0/marginalrevolution/~https://nam11.safelinks.protection.outlook.com/?url=https%3A%2F%2Fwww.thefp.com%2Fforum%2Fpost%2F1%3Fa%3D1&data=05%7C02&reserved=0";
        assert_eq!(unwrapped(wrapped).as_deref(), Some("https://www.thefp.com/forum/post/1?a=1"));
    }

    #[test]
    fn strips_dropped_links_and_images() {
        let rules = LinkRules::default();
        let html = r#"<p>Body <a href="https://example.com/">kept</a></p><div><a title="Like on Facebook" href="https://feeds.feedblitz.com/_/28/972063845/marginalrevolution"><img src="https://assets.feedblitz.com/i/fblike20.png"></a><a href="https://example.com/#comments"><img src="https://assets.feedblitz.com/i/comments20.png"></a></div><img height="1" src="https://feeds.feedblitz.com/~/i/972063845/0/marginalrevolution" width="1">"#;
        let out = rules.strip(html);
        assert!(!out.contains("feedblitz"), "{out}");
        assert!(out.contains(r#"<a href="https://example.com/">kept</a>"#), "{out}");
        assert!(matches!(rules.strip("<p>plain</p>"), Cow::Borrowed(_)));
    }

    #[test]
    fn rejects_bad_rule_files() {
        let src = |t: &'static str| LinkRules::from_sources(&[("test", t)]);
        assert!(src("[[unwrap]]\nname = \"x\"\npattern = '^https://a/(?<u>.+)'").is_err());
        assert!(src("[[drop]]\nname = \"x\"\npattern = '('").is_err());
        assert!(src("[[drop]]\nname = \"x\"\npattern = 'a'\nflavor = 1").is_err());
        assert!(src("[[unwrap]]\npattern = '(?<url>.+)'").is_err());
    }

    #[test]
    fn extra_rules_extend_the_builtin_set() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rules.toml");
        std::fs::write(&path, "[[unwrap]]\nname = \"bounce\"\npattern = '^https://bounce\\.example/\\?to=(?<url>.+)$'\ndecode = true\n").unwrap();
        let rules = LinkRules::load(Some(&path)).unwrap();
        assert_ne!(rules.fingerprint(), LinkRules::default().fingerprint());
        let got = rules.unwrap_url("https://bounce.example/?to=https%3A%2F%2Fexample.com%2Fa").map(String::from);
        assert_eq!(got.as_deref(), Some("https://example.com/a"));
        assert!(rules.unwrap_url(&format!("https://feeds.feedblitz.com/~/t/0/0/mr/~{ARTICLE}")).is_some());
    }
}
