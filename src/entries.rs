use serde::Serialize;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use crate::db::now;
use crate::error::AppResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadFilter {
    #[default]
    Any,
    Unread,
    Read,
}

#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub user_id: i64,
    pub feed_id: Option<i64>,
    pub folder_id: Option<i64>,
    pub read: ReadFilter,
    pub starred: bool,
    pub search: Option<String>,
    pub oldest_first: bool,
    pub cursor: Option<Cursor>,
    pub published_after: Option<i64>,
    pub published_before: Option<i64>,
    pub ids: Option<Vec<i64>>,
    pub limit: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub published_at: i64,
    pub id: i64,
}

impl Cursor {
    pub fn encode(&self) -> String {
        format!("{}-{}", self.published_at, self.id)
    }

    pub fn decode(s: &str) -> Option<Self> {
        let (p, i) = s.split_once('-')?;
        Some(Self { published_at: p.parse().ok()?, id: i.parse().ok()? })
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EntryRow {
    pub id: i64,
    pub feed_id: i64,
    pub feed_title: String,
    pub feed_site_url: Option<String>,
    pub folder_name: Option<String>,
    pub title: String,
    pub url: Option<String>,
    pub author: Option<String>,
    pub content: String,
    pub snippet: String,
    pub enclosure_url: Option<String>,
    pub enclosure_type: Option<String>,
    pub published_at: i64,
    pub created_at: i64,
    pub is_read: bool,
    pub is_starred: bool,
    pub like_uri: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct EntryRef {
    pub id: i64,
    pub feed_id: i64,
    pub published_at: i64,
}

pub fn fts_query(input: &str) -> Option<String> {
    let terms: Vec<String> = input
        .split_whitespace()
        .map(|t| format!("\"{}\"", t.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        return None;
    }
    Some(format!("{}*", terms.join(" ")))
}

fn push_where(qb: &mut QueryBuilder<Sqlite>, f: &Filter) {
    qb.push(" WHERE e.user_id = ").push_bind(f.user_id);
    if let Some(id) = f.feed_id {
        qb.push(" AND e.feed_id = ").push_bind(id);
    }
    if let Some(id) = f.folder_id {
        qb.push(" AND f.folder_id = ").push_bind(id);
    }
    match f.read {
        ReadFilter::Any => {}
        ReadFilter::Unread => {
            qb.push(" AND e.is_read = 0");
        }
        ReadFilter::Read => {
            qb.push(" AND e.is_read = 1");
        }
    }
    if f.starred {
        qb.push(" AND e.is_starred = 1");
    }
    if let Some(q) = f.search.as_deref().and_then(fts_query) {
        qb.push(" AND e.id IN (SELECT rowid FROM entries_fts WHERE entries_fts MATCH ")
            .push_bind(q)
            .push(")");
    }
    if let Some(t) = f.published_after {
        qb.push(" AND e.published_at > ").push_bind(t);
    }
    if let Some(t) = f.published_before {
        qb.push(" AND e.published_at < ").push_bind(t);
    }
    if let Some(ids) = &f.ids {
        qb.push(" AND e.id IN (");
        let mut sep = qb.separated(", ");
        for id in ids {
            sep.push_bind(*id);
        }
        if ids.is_empty() {
            sep.push("NULL");
        }
        qb.push(")");
    }
    if let Some(c) = f.cursor {
        let op = if f.oldest_first { ">" } else { "<" };
        qb.push(format_args!(" AND (e.published_at, e.id) {op} ("))
            .push_bind(c.published_at)
            .push(", ")
            .push_bind(c.id)
            .push(")");
    }
    let dir = if f.oldest_first { "ASC" } else { "DESC" };
    qb.push(format_args!(" ORDER BY e.published_at {dir}, e.id {dir} LIMIT "))
        .push_bind(f.limit + 1);
}

fn split_page<T>(mut rows: Vec<T>, limit: i64, cursor_of: impl Fn(&T) -> Cursor) -> (Vec<T>, Option<Cursor>) {
    if rows.len() as i64 > limit {
        rows.truncate(limit as usize);
        let next = rows.last().map(cursor_of);
        (rows, next)
    } else {
        (rows, None)
    }
}

pub async fn list(db: &SqlitePool, f: &Filter) -> AppResult<(Vec<EntryRow>, Option<Cursor>)> {
    let mut qb = QueryBuilder::new(
        "SELECT e.id, e.feed_id, COALESCE(f.user_title, f.title) AS feed_title, f.site_url AS feed_site_url,
                fo.name AS folder_name, e.title, e.url, e.author, e.content, e.snippet, e.enclosure_url,
                e.enclosure_type, e.published_at, e.created_at, e.is_read, e.is_starred, e.like_uri
         FROM entries e JOIN feeds f ON f.id = e.feed_id LEFT JOIN folders fo ON fo.id = f.folder_id",
    );
    push_where(&mut qb, f);
    let rows = qb.build_query_as::<EntryRow>().fetch_all(db).await?;
    Ok(split_page(rows, f.limit, |r| Cursor { published_at: r.published_at, id: r.id }))
}

pub async fn list_refs(db: &SqlitePool, f: &Filter) -> AppResult<(Vec<EntryRef>, Option<Cursor>)> {
    let mut qb = QueryBuilder::new(
        "SELECT e.id, e.feed_id, e.published_at FROM entries e JOIN feeds f ON f.id = e.feed_id",
    );
    push_where(&mut qb, f);
    let rows = qb.build_query_as::<EntryRef>().fetch_all(db).await?;
    Ok(split_page(rows, f.limit, |r| Cursor { published_at: r.published_at, id: r.id }))
}

pub async fn set_read(db: &SqlitePool, user_id: i64, ids: &[i64], read: bool) -> AppResult<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let mut qb = QueryBuilder::new("UPDATE entries SET is_read = ");
    qb.push_bind(read)
        .push(", read_at = ")
        .push_bind(read.then(now))
        .push(" WHERE user_id = ")
        .push_bind(user_id)
        .push(" AND is_read != ")
        .push_bind(read)
        .push(" AND id IN (");
    let mut sep = qb.separated(", ");
    for id in ids {
        sep.push_bind(*id);
    }
    qb.push(")");
    Ok(qb.build().execute(db).await?.rows_affected())
}

pub async fn set_starred(db: &SqlitePool, user_id: i64, ids: &[i64], starred: bool) -> AppResult<u64> {
    if ids.is_empty() {
        return Ok(0);
    }
    let mut qb = QueryBuilder::new("UPDATE entries SET is_starred = ");
    qb.push_bind(starred)
        .push(", starred_at = ")
        .push_bind(starred.then(now))
        .push(" WHERE user_id = ")
        .push_bind(user_id)
        .push(" AND is_starred != ")
        .push_bind(starred)
        .push(" AND id IN (");
    let mut sep = qb.separated(", ");
    for id in ids {
        sep.push_bind(*id);
    }
    qb.push(")");
    Ok(qb.build().execute(db).await?.rows_affected())
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MarkScope {
    pub feed_id: Option<i64>,
    pub folder_id: Option<i64>,
    pub starred: bool,
    pub created_before: Option<i64>,
    pub published_before: Option<i64>,
}

pub async fn mark_all_read(db: &SqlitePool, user_id: i64, scope: MarkScope) -> AppResult<u64> {
    let mut qb = QueryBuilder::new("UPDATE entries SET is_read = 1, read_at = ");
    qb.push_bind(now())
        .push(" WHERE is_read = 0 AND user_id = ")
        .push_bind(user_id);
    if let Some(id) = scope.feed_id {
        qb.push(" AND feed_id = ").push_bind(id);
    }
    if let Some(id) = scope.folder_id {
        qb.push(" AND feed_id IN (SELECT id FROM feeds WHERE folder_id = ")
            .push_bind(id)
            .push(")");
    }
    if scope.starred {
        qb.push(" AND is_starred = 1");
    }
    if let Some(t) = scope.created_before {
        qb.push(" AND created_at <= ").push_bind(t);
    }
    if let Some(t) = scope.published_before {
        qb.push(" AND published_at <= ").push_bind(t);
    }
    Ok(qb.build().execute(db).await?.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_roundtrip() {
        let c = Cursor { published_at: 1_700_000_000, id: 42 };
        assert_eq!(Cursor::decode(&c.encode()), Some(c));
        assert_eq!(Cursor::decode("garbage"), None);
    }

    #[test]
    fn fts_quotes_terms() {
        assert_eq!(fts_query(r#"rust "async" OR"#).unwrap(), r#""rust" """async""" "OR"*"#);
        assert_eq!(fts_query("   "), None);
    }
}
