use sqlx::Postgres;
use sqlx::Transaction;

use crate::error::DbError;
use crate::repository::UserEntry;
use crate::repository::UserRepository;

pub struct PostgresUserRepository<'a, 'c> {
    pub tx: &'a mut Transaction<'c, Postgres>,
}

impl<'a, 'c> UserRepository for PostgresUserRepository<'a, 'c> {
    async fn insert(&mut self, params: &UserEntry) -> Result<i64, DbError> {
        println!("here");
        let id = sqlx::query_scalar(
            "INSERT INTO users (username, email, password_hash, is_admin, created_at) VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(&params.username)
        .bind(&params.email)
        .bind(&params.password_hash)
        .bind(params.is_admin)
        .bind(params.created_at)
        .fetch_one(&mut **self.tx)
        .await?;
        Ok(id)
    }
}
