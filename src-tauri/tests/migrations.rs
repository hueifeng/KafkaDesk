use sqlx::{sqlite::SqlitePoolOptions, Row};
use std::{
    path::PathBuf,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

fn unique_database_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("traceforge-migrations-{nanos}.sqlite3"))
}

#[tokio::test]
async fn embedded_migrations_create_expected_schema() {
    let database_path = unique_database_path();
    let database_url = format!("sqlite://{}", database_path.display());

    let options = sqlx::sqlite::SqliteConnectOptions::from_str(&database_url)
        .expect("sqlite connect options should parse")
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .expect("sqlite pool should connect");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("embedded migrations should succeed");

    let tables = sqlx::query("SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?")
        .bind("correlation_rules")
        .fetch_all(&pool)
        .await
        .expect("table lookup should succeed");

    assert_eq!(tables.len(), 1, "correlation_rules table should exist");

    let cluster_profile_columns = sqlx::query("PRAGMA table_info(cluster_profiles)")
        .fetch_all(&pool)
        .await
        .expect("cluster_profiles table info should load")
        .into_iter()
        .map(|row| {
            row.try_get::<String, _>("name")
                .expect("column row should include name")
        })
        .collect::<Vec<_>>();

    for expected_column in [
        "auth_credential_ref",
        "tls_ca_cert_path",
        "tls_client_cert_path",
        "tls_client_key_path",
    ] {
        assert!(
            cluster_profile_columns
                .iter()
                .any(|column| column == expected_column),
            "expected cluster_profiles.{expected_column} to exist, got {cluster_profile_columns:?}"
        );
    }

    pool.close().await;
    std::fs::remove_file(&database_path).expect("temporary sqlite database should be removable");
}
