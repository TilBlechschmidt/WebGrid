use chrono::{Duration, TimeZone, Utc};
use sqlx::{
    cursor::Cursor,
    error::Error as SQLError,
    executor::RefExecutor,
    sqlite::{Sqlite, SqliteQueryAs},
    Executor, Row, SqliteConnection,
};
use std::path::PathBuf;

pub async fn delete_all_files<E: Executor<Database = Sqlite>>(
    con: &mut E,
) -> Result<u64, SQLError> {
    sqlx::query!("DELETE FROM Files").execute(con).await
}

pub async fn delete_file<E: Executor<Database = Sqlite>>(
    con: &mut E,
    path: &str,
) -> Result<u64, SQLError> {
    sqlx::query!("DELETE FROM Files WHERE Path LIKE ?", path)
        .execute(con)
        .await
}

pub async fn insert_file<E: Executor<Database = Sqlite>>(
    path: &str,
    metadata: Option<std::fs::Metadata>,
    con: &mut E,
) -> Result<(), SQLError> {
    let mut size: f64 = 0.0;
    let mut last_modified = Utc::now();
    let mut last_access = Utc::now();

    // Consider dates that are before 2000 or more than 24 hours in the future to be invalid.
    let past_sanity_date = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0);
    let future_sanity_date = Utc::now() + Duration::hours(24);

    if let Some(meta) = metadata {
        size = meta.len() as f64;

        if let Ok(modified) = meta.modified() {
            let date_time = modified.into();
            if date_time > past_sanity_date && date_time < future_sanity_date {
                last_modified = date_time;
            }
        }

        if let Ok(accessed) = meta.accessed() {
            let date_time = accessed.into();
            if date_time > past_sanity_date && date_time < future_sanity_date {
                last_access = date_time;
            }
        }
    }

    sqlx::query!(
        r#"
            INSERT OR REPLACE INTO Files ( Path, Size, ModificationTime, LastAccessTime )
            VALUES ( $1, $2, $3, $4 )
        "#,
        path,
        size,
        last_modified.to_rfc3339(),
        last_access.to_rfc3339()
    )
    .execute(con)
    .await?;

    Ok(())
}

pub async fn used_bytes<'e, E>(executor: E) -> Result<f64, SQLError>
where
    E: 'e + Send + RefExecutor<'e, Database = Sqlite>,
{
    // let mut cursor = sqlx::query(r#"SELECT SUM(Size) FROM Files"#).fetch(executor);
    // Ok(cursor.next().await?)

    let row: (f64,) = sqlx::query_as("SELECT SUM(Size) FROM Files")
        .fetch_one(executor)
        .await?;

    Ok(row.0)
}

pub async fn files_to_delete<'e, E>(executor: E, target_size: f64) -> Result<Vec<PathBuf>, SQLError>
where
    E: RefExecutor<'e, Database = Sqlite>,
{
    let mut cursor = sqlx::query("SELECT Path, CumulativeSize FROM Eviction").fetch(executor);
    let mut paths = Vec::new();

    while let Some(row) = cursor.next().await? {
        let path: String = row.get("Path");
        let cumulative_size: f64 = row.get("CumulativeSize");

        paths.push(PathBuf::from(path));

        if cumulative_size >= target_size {
            break;
        }
    }

    Ok(paths)
}

pub async fn setup_tables(mut con: &mut SqliteConnection) -> Result<(), SQLError> {
    // Create tables
    sqlx::query_file!("src/sql/schema.sql")
        .execute(&mut con)
        .await?;

    Ok(())
}

pub async fn setup_views(mut con: &mut SqliteConnection, formula: &str) -> Result<(), SQLError> {
    // Create views
    // Ignore files that have been modified in the last 5 minutes
    let views = format!(
        include_str!("sql/views.sql"),
        score_formula = parse_score_formula(formula),
        seconds_since_modification_threshold = 300
    );
    sqlx::query(&views).execute(&mut con).await?;

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
