use decodering_core::error::DbError;
use decodering_core::repository::User;
use decodering_core::repository::UserEntry;
use decodering_core::repository::UserRepository;
use sqlx::Sqlite;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::UserRow;

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

    async fn get_by_api_key(&mut self, api_key: &str) -> Result<Option<User>, DbError> {
        let user: Option<UserRow> = sqlx::query_as::<_, UserRow>(
            "SELECT u.id, u.username, u.email, u.password_hash, u.is_admin, u.created_at
            FROM users u
            INNER JOIN api_keys k ON k.user_id = u.id
            WHERE k.api_key = ?
              AND (k.expires_at IS NULL OR k.expires_at > unixepoch())",
        )
        .bind(api_key)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(user.map(Into::into))
    }
}
