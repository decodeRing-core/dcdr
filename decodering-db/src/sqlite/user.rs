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

impl UserRepository for SqliteUserRepository<'_> {
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

    async fn get_by_username(&mut self, username: &str) -> Result<Option<User>, DbError> {
        let user: Option<UserRow> = sqlx::query_as::<_, UserRow>(
            "SELECT u.id, u.username, u.email, u.password_hash, u.is_admin, u.created_at
            FROM users u
            WHERE u.username = ?",
        )
        .bind(username)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(user.map(Into::into))
    }

    async fn update(
        &mut self,
        id: i64,
        email: &str,
        is_admin: bool,
        password_hash: Option<&str>,
    ) -> Result<u64, DbError> {
        let rows = sqlx::query(
            "UPDATE users SET \
                email = ?, \
                is_admin = ?, \
                password_hash = COALESCE(?, password_hash) \
             WHERE id = ?",
        )
        .bind(email)
        .bind(is_admin)
        .bind(password_hash)
        .bind(id)
        .execute(&mut **self.tx)
        .await
        .map_err(map_sqlx)?
        .rows_affected();
        Ok(rows)
    }

    async fn delete(&mut self, id: i64) -> Result<u64, DbError> {
        let rows = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&mut **self.tx)
            .await
            .map_err(map_sqlx)?
            .rows_affected();
        Ok(rows)
    }

    async fn get_by_api_key(&mut self, api_key_hash: &str) -> Result<Option<User>, DbError> {
        let user: Option<UserRow> = sqlx::query_as::<_, UserRow>(
            "SELECT u.id, u.username, u.email, u.password_hash, u.is_admin, u.created_at
            FROM users u
            INNER JOIN api_keys k ON k.user_id = u.id
            WHERE k.key_hash = ?
              AND (k.expires_at IS NULL OR k.expires_at > unixepoch())",
        )
        .bind(api_key_hash)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(user.map(Into::into))
    }

    async fn get_admin_by_api_key(&mut self, api_key_hash: &str) -> Result<Option<User>, DbError> {
        let user: Option<UserRow> = sqlx::query_as::<_, UserRow>(
            "SELECT u.id, u.username, u.email, u.password_hash, u.is_admin, u.created_at
            FROM users u
            INNER JOIN api_keys k ON k.user_id = u.id
            WHERE k.key_hash = ?
              AND (k.expires_at IS NULL OR k.expires_at > unixepoch()) AND u.is_admin",
        )
        .bind(api_key_hash)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(user.map(Into::into))
    }
}
