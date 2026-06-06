use sqlx::PgPool;
use sqlx::Pool;
use sqlx::SqlitePool;
use sqlx::migrate::{Migrate, Migrator};

static SQLITE_MIGRATOR: Migrator = sqlx::migrate!("./migrations/sqlite");
static POSTGRES_MIGRATOR: Migrator = sqlx::migrate!("./migrations/postgres");

pub async fn migrate_sqlite(pool: &SqlitePool, auto_migrate: bool) -> std::io::Result<()> {
    run_or_check(&SQLITE_MIGRATOR, pool, auto_migrate).await
}

pub async fn migrate_postgres(pool: &PgPool, auto_migrate: bool) -> std::io::Result<()> {
    run_or_check(&POSTGRES_MIGRATOR, pool, auto_migrate).await
}

async fn run_or_check<T>(
    migrator: &Migrator,
    pool: &Pool<T>,
    auto_migrate: bool,
) -> std::io::Result<()>
where
    T: sqlx::Database,
    T::Connection: sqlx::migrate::Migrate,
{
    if auto_migrate {
        migrator.run(pool).await.map_err(std::io::Error::other)?;
    } else {
        let pending = pending_migrations(migrator, pool)
            .await
            .map_err(std::io::Error::other)?;
        if !pending.is_empty() {
            return Err(std::io::Error::other(format!(
                "pending migrations, refusing to start: {pending:?}"
            )));
        }
    }
    Ok(())
}

async fn pending_migrations<T>(migrator: &Migrator, pool: &Pool<T>) -> Result<Vec<i64>, sqlx::Error>
where
    T: sqlx::Database,
    T::Connection: Migrate,
{
    const TABLE: &str = "_sqlx_migrations";

    let mut conn = pool.acquire().await?;
    conn.ensure_migrations_table(TABLE).await?;

    let applied: std::collections::HashSet<i64> = conn
        .list_applied_migrations(TABLE)
        .await?
        .into_iter()
        .map(|m| m.version)
        .collect();

    Ok(migrator
        .iter()
        .filter(|m| !applied.contains(&m.version))
        .map(|m| m.version)
        .collect())
}
