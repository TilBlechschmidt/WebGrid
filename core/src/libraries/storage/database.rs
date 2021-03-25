use crate::libraries::storage::FileMetadata;
use futures::StreamExt;
use sqlx::{error::Error as SQLError, sqlite::Sqlite, Executor, Row, SqliteConnection};
use std::path::PathBuf;

pub async fn delete_all_files<'e, E>(con: E) -> Result<u64, SQLError>
where
    E: Executor<'e, Database = Sqlite>,
{
    Ok(sqlx::query!("DELETE FROM Files")
        .execute(con)
        .await?
        .rows_affected())
}

pub async fn delete_file<'e, E>(con: E, path: &str) -> Result<u64, SQLError>
where
    E: Executor<'e, Database = Sqlite>,
{
    Ok(sqlx::query!("DELETE FROM Files WHERE Path LIKE ?", path)
        .execute(con)
        .await?
        .rows_affected())
}

pub async fn insert_file<'e, E>(metadata: FileMetadata, con: E) -> Result<(), SQLError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let last_modified_string = metadata.last_modified.to_rfc3339();
    let last_access_string = metadata.last_access.to_rfc3339();
    let path = metadata.path.to_str().unwrap_or_default();

    sqlx::query!(
        r#"
            INSERT OR REPLACE INTO Files ( Path, Size, ModificationTime, LastAccessTime )
            VALUES ( $1, $2, $3, $4 )
        "#,
        path,
        metadata.size,
        last_modified_string,
        last_access_string
    )
    .execute(con)
    .await?;

    Ok(())
}

pub async fn used_bytes<'e, E>(executor: E) -> Result<f64, SQLError>
where
    E: 'e + Send + Executor<'e, Database = Sqlite>,
{
    let row: (f64,) = sqlx::query_as("SELECT SUM(Size) FROM Files")
        .fetch_one(executor)
        .await?;

    Ok(row.0)
}

pub async fn files_to_delete<'e, E>(executor: E, target_size: f64) -> Result<Vec<PathBuf>, SQLError>
where
    E: Executor<'e, Database = Sqlite>,
{
    let mut cursor = sqlx::query("SELECT Path, CumulativeSize FROM Eviction").fetch(executor);
    let mut paths = Vec::new();

    while let Some(r) = cursor.next().await {
        let row = r?;
        let path: String = row.get("Path");
        let cumulative_size: f64 = row.get("CumulativeSize");

        paths.push(PathBuf::from(path));

        if cumulative_size >= target_size {
            break;
        }
    }

    Ok(paths)
}

pub async fn setup_tables(con: &mut SqliteConnection) -> Result<(), SQLError> {
    // Create tables
    sqlx::query_file!("src/libraries/storage/sql/schema.sql")
        .execute(con)
        .await?;

    Ok(())
}

pub async fn setup_views(con: &mut SqliteConnection, formula: &str) -> Result<(), SQLError> {
    // Create views
    // Ignore files that have been modified in the last 5 minutes
    let views = format!(
        include_str!("sql/views.sql"),
        score_formula = parse_score_formula(formula),
        seconds_since_modification_threshold = 300
    );
    sqlx::query(&views).execute(con).await?;

    Ok(())
}

fn parse_score_formula(formula: &str) -> String {
    formula
        .replace(
            "ModificationTime",
            "(strftime('%s', 'now') - strftime('%s', ModificationTime))",
        )
        .replace(
            "LastAccessTime",
            "(strftime('%s', 'now') - strftime('%s', LastAccessTime))",
        )
}
