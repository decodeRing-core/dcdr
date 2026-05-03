use decodering_core::error::DbError;
use decodering_core::repository::UserEntry;
use decodering_core::repository::UserRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;

pub struct SqliteUserRepository<'a> {
    pub tx: &'a mut Transaction<'static, Sqlite>,
}

impl<'a> UserRepository for SqliteUserRepository<'a> {
    async fn insert(&mut self, params: &UserEntry) -> Result<i64, DbError> {
        let id = sqlx::query_scalar::<_, i64>(
            "INSERT INTO users (username, email, password_hash, is_admin, created_at) \
             VALUES (?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(&params.username)
        .bind(&params.email)
        .bind(&params.password_hash)
        .bind(params.is_admin)
        .bind(params.created_at)
        .fetch_one(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(id)
    }
}
