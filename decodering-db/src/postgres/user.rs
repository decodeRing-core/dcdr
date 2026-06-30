use decodering_core::error::DbError;
use decodering_core::repository::User;
use decodering_core::repository::UserEntry;
use decodering_core::repository::UserRepository;
use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::map_sqlx;
use crate::repository::UserRow;

pub struct PostgresUserRepository<'a> {
    pub tx: &'a mut Transaction<'static, Postgres>,
}

impl UserRepository for PostgresUserRepository<'_> {
    async fn insert(&mut self, params: &UserEntry) -> Result<i64, DbError> {
        let id = sqlx::query_scalar(
            "INSERT INTO users (username, email, password_hash, is_admin, created_at) VALUES ($1, $2, $3, $4, $5) RETURNING id",
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
            WHERE u.username = $1",
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
                email = $1, \
                is_admin = $2, \
                password_hash = COALESCE($3, password_hash) \
             WHERE id = $4",
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
        let rows = sqlx::query("DELETE FROM users WHERE id = $1")
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
            WHERE k.key_hash = $1
              AND (k.expires_at IS NULL OR k.expires_at > EXTRACT(EPOCH FROM NOW())::BIGINT)",
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
            WHERE k.key_hash = $1
              AND (k.expires_at IS NULL OR k.expires_at > EXTRACT(EPOCH FROM NOW())::BIGINT)
              AND u.is_admin = true",
        )
        .bind(api_key_hash)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(map_sqlx)?;
        Ok(user.map(Into::into))
    }
}
