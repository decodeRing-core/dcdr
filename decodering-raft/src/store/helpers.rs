// Helpers

//use std::io;

// use sqlx::{SqliteConnection, SqlitePool, Transaction};

// pub(crate) async fn set_meta(
//     tx: &mut Transaction<'_, sqlx::Sqlite>,
//     key: &str,
//     value: &str,
// ) -> Result<(), io::Error> {
//     sqlx::query(
//         "INSERT INTO meta (key, value) VALUES (?, ?)
//          ON CONFLICT(key) DO UPDATE SET value = excluded.value",
//     )
//     .bind(key)
//     .bind(value)
//     .execute(&mut **tx)
//     .await
//     .map_err(io::Error::other)?;
//     Ok(())
// }

// pub(crate) async fn get_meta(pool: &SqlitePool, key: &str) -> Result<Option<String>, io::Error> {
//     let row: Option<(String,)> = sqlx::query_as("SELECT value FROM meta WHERE key = ?")
//         .bind(key)
//         .fetch_optional(pool)
//         .await
//         .map_err(io::Error::other)?;
//     Ok(row.map(|(v,)| v))
// }

// pub(crate) async fn get_meta_with_conn(
//     conn: &mut SqliteConnection,
//     key: &str,
// ) -> Result<Option<String>, io::Error> {
//     let row: Option<(String,)> = sqlx::query_as("SELECT value FROM meta WHERE key = ?")
//         .bind(key)
//         .fetch_optional(conn)
//         .await
//         .map_err(io::Error::other)?;
//     Ok(row.map(|(v,)| v))
// }
