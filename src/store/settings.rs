use anyhow::Result;
use serde_json::Value;
use sqlx::SqlitePool;

/// Return all app settings as `(key, value)` pairs, ordered by key.
pub async fn get_all(pool: &SqlitePool) -> Result<Vec<(String, Value)>> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT key, value FROM app_settings ORDER BY key")
            .fetch_all(pool)
            .await?;

    rows.into_iter()
        .map(|(k, v)| serde_json::from_str::<Value>(&v).map(|val| (k, val)))
        .collect::<Result<_, _>>()
        .map_err(anyhow::Error::from)
}

/// Upsert a batch of settings in a single transaction.
pub async fn set_many(pool: &SqlitePool, entries: &[(&str, &Value)]) -> Result<()> {
    let mut tx = pool.begin().await?;
    for (key, value) in entries {
        let json = serde_json::to_string(value)?;
        sqlx::query(
            "INSERT INTO app_settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value     = excluded.value,
                                            updated_at = datetime('now')",
        )
        .bind(key)
        .bind(&json)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::testutil::ephemeral_store;

    #[tokio::test]
    async fn round_trip_single_setting() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let val = serde_json::json!("dark");
        set_many(pool, &[("theme", &val)]).await.unwrap();

        let all = get_all(pool).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, "theme");
        assert_eq!(all[0].1, val);
    }

    #[tokio::test]
    async fn upsert_overwrites_existing() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let v1 = serde_json::json!(50);
        let v2 = serde_json::json!(100);
        set_many(pool, &[("page_size", &v1)]).await.unwrap();
        set_many(pool, &[("page_size", &v2)]).await.unwrap();

        let all = get_all(pool).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].1, v2);
    }

    #[tokio::test]
    async fn batch_insert_is_transactional() {
        let (store, _dir) = ephemeral_store().await;
        let pool = store.pool();

        let a = serde_json::json!("stable");
        let b = serde_json::json!(true);
        set_many(pool, &[("channel", &a), ("notifications", &b)])
            .await
            .unwrap();

        let all = get_all(pool).await.unwrap();
        assert_eq!(all.len(), 2);
    }
}
