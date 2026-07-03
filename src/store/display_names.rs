use anyhow::Result;
use sqlx::SqlitePool;

const MAX_DISPLAY_NAME_LEN: usize = 64;

/// A single stored table display-name override.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayNameEntry {
    pub schema_name: String,
    pub table_name: String,
    pub display_name: String,
}

/// List all stored display-name overrides, across all schemas/tables.
pub async fn list_display_names(pool: &SqlitePool) -> Result<Vec<DisplayNameEntry>> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT schema_name, table_name, display_name
         FROM table_display_names
         ORDER BY schema_name, table_name",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(schema_name, table_name, display_name)| DisplayNameEntry {
            schema_name,
            table_name,
            display_name,
        })
        .collect())
}

/// Look up a single stored display-name override, if any.
pub async fn get_display_name(
    pool: &SqlitePool,
    schema_name: &str,
    table_name: &str,
) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT display_name FROM table_display_names WHERE schema_name = ? AND table_name = ?",
    )
    .bind(schema_name)
    .bind(table_name)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(display_name,)| display_name))
}

/// Set (upsert) a table's display-name override.
///
/// The name is trimmed before storage. An empty-after-trim name deletes the
/// override instead (revert-to-automatic-name semantics). Names longer than
/// [`MAX_DISPLAY_NAME_LEN`] (after trimming) are rejected.
pub async fn set_display_name(
    pool: &SqlitePool,
    schema_name: &str,
    table_name: &str,
    display_name: &str,
) -> Result<()> {
    let trimmed = display_name.trim();

    if trimmed.is_empty() {
        sqlx::query(
            "DELETE FROM table_display_names WHERE schema_name = ? AND table_name = ?",
        )
        .bind(schema_name)
        .bind(table_name)
        .execute(pool)
        .await?;
        return Ok(());
    }

    if trimmed.len() > MAX_DISPLAY_NAME_LEN {
        anyhow::bail!("display name exceeds maximum length of {MAX_DISPLAY_NAME_LEN} characters");
    }

    sqlx::query(
        "INSERT INTO table_display_names (schema_name, table_name, display_name, updated_at)
         VALUES (?, ?, ?, datetime('now'))
         ON CONFLICT(schema_name, table_name)
         DO UPDATE SET display_name = excluded.display_name, updated_at = excluded.updated_at",
    )
    .bind(schema_name)
    .bind(table_name)
    .bind(trimmed)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::testutil::ephemeral_store;

    #[tokio::test]
    async fn set_and_get_display_name_round_trip() {
        let (store, _dir) = ephemeral_store().await;
        set_display_name(store.pool(), "public", "vehicles", "Vehicle Telemetry")
            .await
            .unwrap();

        let got = get_display_name(store.pool(), "public", "vehicles")
            .await
            .unwrap();
        assert_eq!(got.as_deref(), Some("Vehicle Telemetry"));
    }

    #[tokio::test]
    async fn set_display_name_overwrites_existing() {
        let (store, _dir) = ephemeral_store().await;
        set_display_name(store.pool(), "public", "vehicles", "First Name")
            .await
            .unwrap();
        set_display_name(store.pool(), "public", "vehicles", "Second Name")
            .await
            .unwrap();

        let got = get_display_name(store.pool(), "public", "vehicles")
            .await
            .unwrap();
        assert_eq!(got.as_deref(), Some("Second Name"));

        // Only one row should exist for this schema/table pair.
        let all = list_display_names(store.pool()).await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn set_display_name_trims_whitespace() {
        let (store, _dir) = ephemeral_store().await;
        set_display_name(store.pool(), "public", "vehicles", "  Padded Name  ")
            .await
            .unwrap();

        let got = get_display_name(store.pool(), "public", "vehicles")
            .await
            .unwrap();
        assert_eq!(got.as_deref(), Some("Padded Name"));
    }

    #[tokio::test]
    async fn set_display_name_empty_after_trim_deletes_row() {
        let (store, _dir) = ephemeral_store().await;
        set_display_name(store.pool(), "public", "vehicles", "Vehicle Telemetry")
            .await
            .unwrap();
        set_display_name(store.pool(), "public", "vehicles", "   ")
            .await
            .unwrap();

        let got = get_display_name(store.pool(), "public", "vehicles")
            .await
            .unwrap();
        assert_eq!(got, None);

        let all = list_display_names(store.pool()).await.unwrap();
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn set_display_name_rejects_over_length() {
        let (store, _dir) = ephemeral_store().await;
        let too_long = "x".repeat(MAX_DISPLAY_NAME_LEN + 1);

        let error = set_display_name(store.pool(), "public", "vehicles", &too_long)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("maximum length"));

        // Rejected write must not have persisted anything.
        let got = get_display_name(store.pool(), "public", "vehicles")
            .await
            .unwrap();
        assert_eq!(got, None);
    }

    #[tokio::test]
    async fn set_display_name_accepts_exact_max_length() {
        let (store, _dir) = ephemeral_store().await;
        let exact = "x".repeat(MAX_DISPLAY_NAME_LEN);

        set_display_name(store.pool(), "public", "vehicles", &exact)
            .await
            .unwrap();

        let got = get_display_name(store.pool(), "public", "vehicles")
            .await
            .unwrap();
        assert_eq!(got.as_deref(), Some(exact.as_str()));
    }

    #[tokio::test]
    async fn list_display_names_returns_all_entries_sorted() {
        let (store, _dir) = ephemeral_store().await;
        set_display_name(store.pool(), "public", "zoo", "Zoo Table")
            .await
            .unwrap();
        set_display_name(store.pool(), "public", "alpha", "Alpha Table")
            .await
            .unwrap();
        set_display_name(store.pool(), "reporting", "alpha", "Reporting Alpha")
            .await
            .unwrap();

        let all = list_display_names(store.pool()).await.unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].schema_name, "public");
        assert_eq!(all[0].table_name, "alpha");
        assert_eq!(all[1].schema_name, "public");
        assert_eq!(all[1].table_name, "zoo");
        assert_eq!(all[2].schema_name, "reporting");
    }

    #[tokio::test]
    async fn get_display_name_missing_returns_none() {
        let (store, _dir) = ephemeral_store().await;
        let got = get_display_name(store.pool(), "public", "nope")
            .await
            .unwrap();
        assert_eq!(got, None);
    }

    #[tokio::test]
    async fn delete_of_nonexistent_override_is_a_noop() {
        let (store, _dir) = ephemeral_store().await;
        // Deleting (empty display name) for a table that has no override should not error.
        set_display_name(store.pool(), "public", "vehicles", "")
            .await
            .unwrap();
        let all = list_display_names(store.pool()).await.unwrap();
        assert!(all.is_empty());
    }
}
