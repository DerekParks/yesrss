use quick_xml::{Reader, XmlVersion};
use quick_xml::escape::escape;
use quick_xml::events::{BytesStart, Event};
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpmlFeed {
    pub url: String,
    pub title: Option<String>,
    pub folder: Option<String>,
}

fn attr(e: &BytesStart, name: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref().eq_ignore_ascii_case(name))
        .and_then(|a| a.normalized_value(XmlVersion::Implicit1_0).ok())
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

/// Flattens nested outlines: a feed belongs to its outermost enclosing folder.
pub fn parse(xml: &str) -> AppResult<Vec<OpmlFeed>> {
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<Option<String>> = Vec::new();
    let mut feeds = Vec::new();
    let mut saw_opml = false;
    loop {
        let event = reader
            .read_event()
            .map_err(|e| AppError::bad(format!("invalid OPML: {e}")))?;
        match event {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let is_empty = matches!(event, Event::Empty(_));
                match e.name().as_ref() {
                    "opml" => saw_opml = true,
                    "outline" => {
                        let title = attr(e, "title").or_else(|| attr(e, "text"));
                        if let Some(url) = attr(e, "xmlUrl") {
                            let folder = stack.iter().flatten().next().cloned();
                            feeds.push(OpmlFeed { url, title, folder });
                            if !is_empty {
                                stack.push(None);
                            }
                        } else if !is_empty {
                            stack.push(title);
                        }
                    }
                    _ => {}
                }
            }
            Event::End(ref e) if e.name().as_ref() == "outline" => {
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !saw_opml {
        return Err(AppError::bad("not an OPML document"));
    }
    Ok(feeds)
}

pub async fn export(db: &SqlitePool, user_id: i64, username: &str) -> AppResult<String> {
    let rows: Vec<(String, Option<String>, String, Option<String>)> = sqlx::query_as(
        "SELECT COALESCE(f.user_title, f.title), f.site_url, f.url, fo.name
         FROM feeds f LEFT JOIN folders fo ON fo.id = f.folder_id
         WHERE f.user_id = ? ORDER BY fo.name IS NOT NULL, fo.name COLLATE NOCASE, 1 COLLATE NOCASE",
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;

    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<opml version=\"2.0\">\n");
    out.push_str(&format!(
        "  <head>\n    <title>{} subscriptions in yesrss</title>\n  </head>\n  <body>\n",
        escape(username)
    ));
    let outline = |title: &str, site: &Option<String>, url: &str, indent: &str| {
        let site = site
            .as_deref()
            .map(|s| format!(" htmlUrl=\"{}\"", escape(s)))
            .unwrap_or_default();
        format!(
            "{indent}<outline type=\"rss\" text=\"{t}\" title=\"{t}\" xmlUrl=\"{u}\"{site}/>\n",
            t = escape(title),
            u = escape(url),
        )
    };
    let mut current: Option<&str> = None;
    for (title, site, url, folder) in &rows {
        if folder.as_deref() != current {
            if current.is_some() {
                out.push_str("    </outline>\n");
            }
            if let Some(name) = folder {
                out.push_str(&format!("    <outline text=\"{n}\" title=\"{n}\">\n", n = escape(name)));
            }
            current = folder.as_deref();
        }
        let indent = if current.is_some() { "      " } else { "    " };
        out.push_str(&outline(title, site, url, indent));
    }
    if current.is_some() {
        out.push_str("    </outline>\n");
    }
    out.push_str("  </body>\n</opml>\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_and_flat() {
        let xml = r#"<?xml version="1.0"?>
<opml version="1.0"><head><title>x</title></head><body>
  <outline text="Tech" title="Tech">
    <outline type="rss" text="A &amp; B" xmlUrl="https://a.example/feed"/>
    <outline text="Deeper"><outline xmlUrl="https://deep.example/rss" text="Deep"/></outline>
  </outline>
  <outline type="rss" title="Loose" xmlurl="https://loose.example/atom"></outline>
  <outline text="Empty folder"/>
</body></opml>"#;
        let feeds = parse(xml).unwrap();
        assert_eq!(
            feeds,
            vec![
                OpmlFeed { url: "https://a.example/feed".into(), title: Some("A & B".into()), folder: Some("Tech".into()) },
                OpmlFeed { url: "https://deep.example/rss".into(), title: Some("Deep".into()), folder: Some("Tech".into()) },
                OpmlFeed { url: "https://loose.example/atom".into(), title: Some("Loose".into()), folder: None },
            ]
        );
    }

    #[test]
    fn rejects_non_opml() {
        assert!(parse("<rss></rss>").is_err());
        assert!(parse("<opml><body><outline").is_err());
    }
}
