use yesrss::db::{begin_write, connect};

#[tokio::test]
async fn pool_recovers_after_sqlite_ends_a_transaction_on_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let db = connect(&dir.path().join("test.db")).await.unwrap();

    for _ in 0..16 {
        let mut tx = begin_write(&db).await.unwrap();
        sqlx::query("ROLLBACK").execute(&mut *tx).await.unwrap();
    }

    for _ in 0..16 {
        let mut tx = begin_write(&db).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS t (x)").execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
    }
}
