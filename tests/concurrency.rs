use yesrss::entries::{self, MarkScope};
use yesrss::feeds::store_entries;
use yesrss::parse::NewEntry;

fn entry(feed: i64, round: usize, i: usize) -> NewEntry {
    NewEntry {
        guid: format!("{feed}-{round}-{i}"),
        url: Some(format!("https://example.com/{feed}/{round}/{i}")),
        title: format!("Post {round}.{i}"),
        author: None,
        content: "<p>body</p>".into(),
        snippet: "body".into(),
        search_text: "body".into(),
        enclosure_url: None,
        enclosure_type: None,
        published_at: 1_700_000_000 + i as i64,
        content_hash: format!("{round}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn writes_succeed_while_feeds_refresh() {
    let dir = tempfile::tempdir().unwrap();
    let db = yesrss::db::connect(&dir.path().join("test.db")).await.unwrap();
    let user = yesrss::auth::create_user(&db, "alice", "correct horse battery").await.unwrap();
    let mut feeds = Vec::new();
    for n in 0..8 {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO feeds (user_id, url, title, created_at) VALUES (?, ?, 'f', 0) RETURNING id",
        )
        .bind(user.id)
        .bind(format!("https://example.com/{n}.xml"))
        .fetch_one(&db)
        .await
        .unwrap();
        feeds.push(id);
    }

    let writers: Vec<_> = feeds
        .iter()
        .map(|&feed| {
            let db = db.clone();
            tokio::spawn(async move {
                for round in 0..30 {
                    let batch: Vec<_> = (0..40).map(|i| entry(feed, round % 3, i)).collect();
                    store_entries(&db, feed, user.id, &batch, round as i64, None).await?;
                    sqlx::query(
                        "INSERT INTO favicons (feed_id, content_type, data, fetched_at) VALUES (?, NULL, NULL, ?)
                         ON CONFLICT (feed_id) DO UPDATE SET fetched_at = excluded.fetched_at",
                    )
                    .bind(feed)
                    .bind(round as i64)
                    .execute(&db)
                    .await?;
                }
                Ok::<_, yesrss::error::AppError>(())
            })
        })
        .collect();

    for _ in 0..60 {
        let scope = MarkScope { created_before: Some(i64::MAX), ..Default::default() };
        entries::mark_all_read(&db, user.id, scope).await.expect("mark all read");
        tokio::task::yield_now().await;
    }
    for w in writers {
        w.await.unwrap().expect("feed refresh");
    }
}
