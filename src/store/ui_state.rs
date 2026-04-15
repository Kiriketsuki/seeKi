/// Generic key/value UI state per connection.
///
/// Schema is live; frontend migration from localStorage is deferred (marked `(future)` in #73).
/// API endpoints are wired and ready — call them once the frontend migrates.
use anyhow::Result;
use serde_json::Value;
use sqlx::SqlitePool;

pub async fn get(pool: &SqlitePool, conn_id: &str, key: &str) -> Result<Option<Value>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT value FROM ui_state WHERE connection_id = ? AND key = ?")
            .bind(conn_id)
            .bind(key)
            .fetch_optional(pool)
            .await?;

    match row {
        None => Ok(None),
        Some((v,)) => Ok(Some(serde_json::from_str(&v)?)),
    }
}

pub async fn set(pool: &SqlitePool, conn_id: &str, key: &str, value: &Value) -> Result<()> {
    let json = serde_json::to_string(value)?;
    sqlx::query(
        "INSERT INTO ui_state (connection_id, key, value) VALUES (?, ?, ?)
         ON CONFLICT(connection_id, key) DO UPDATE SET value     = excluded.value,
                                                       updated_at = datetime('now')",
    )
    .bind(conn_id)
    .bind(key)
    .bind(&json)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::testutil::ephemeral_store;

    #[tokio::test]
    async fn get_missing_returns_none() {
        let (store, _dir) = ephemeral_store().await;
        let result = get(store.pool(), "conn", "sidebar_collapsed").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn set_and_get_round_trip() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let val = serde_json::json!(true);
        set(pool, "conn", "sidebar_collapsed", &val).await.unwrap();
        let got = get(pool, "conn", "sidebar_collapsed").await.unwrap();
        assert_eq!(got, Some(val));
    }

    #[tokio::test]
    async fn set_overwrites_existing() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let v1 = serde_json::json!(false);
        let v2 = serde_json::json!(true);
        set(pool, "conn", "sidebar_collapsed", &v1).await.unwrap();
        set(pool, "conn", "sidebar_collapsed", &v2).await.unwrap();
        let got = get(pool, "conn", "sidebar_collapsed").await.unwrap();
        assert_eq!(got, Some(v2));
    }
}
