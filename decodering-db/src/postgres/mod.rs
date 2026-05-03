use std::str::FromStr;

use decodering_core::error::DbError;
use decodering_core::tx::{Database, Tx};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Postgres, Transaction};

use crate::error::map_sqlx;
use crate::postgres::api_key::PostgresApiKeysRepository;
use crate::postgres::app::PostgresAppRepository;
use crate::postgres::audit::PostgresAuditRepository;
use crate::postgres::principal::PostgresPrincipalRepository;
use crate::postgres::secret_mapping::PostgresSecretMappingRepository;
use crate::postgres::shamir::PostgresShamirRepository;
use crate::postgres::user::PostgresUserRepository;

mod api_key;
mod app;
mod audit;
mod principal;
mod secret_mapping;
mod shamir;
mod user;

pub struct PostgresTx<'c> {
    tx: Transaction<'c, Postgres>,
}

impl<'c> Tx for PostgresTx<'c> {
    type PrincipalRepo<'a>
        = PostgresPrincipalRepository<'a, 'c>
    where
        Self: 'a;
    type AuditRepo<'a>
        = PostgresAuditRepository<'a, 'c>
    where
        Self: 'a;
    type ShamirRepo<'a>
        = PostgresShamirRepository<'a, 'c>
    where
        Self: 'a;
    type ApiKeysRepo<'a>
        = PostgresApiKeysRepository<'a, 'c>
    where
        Self: 'a;
    type UserRepo<'a>
        = PostgresUserRepository<'a, 'c>
    where
        Self: 'a;
    type AppRepo<'a>
        = PostgresAppRepository<'a, 'c>
    where
        Self: 'a;
    type SecretMappingRepo<'a>
        = PostgresSecretMappingRepository<'a, 'c>
    where
        Self: 'a;

    fn principal(&mut self) -> PostgresPrincipalRepository<'_, 'c> {
        PostgresPrincipalRepository { tx: &mut self.tx }
    }

    fn audit(&mut self) -> PostgresAuditRepository<'_, 'c> {
        PostgresAuditRepository { tx: &mut self.tx }
    }

    fn shamir(&mut self) -> PostgresShamirRepository<'_, 'c> {
        PostgresShamirRepository { tx: &mut self.tx }
    }

    fn api_key(&mut self) -> PostgresApiKeysRepository<'_, 'c> {
        PostgresApiKeysRepository { tx: &mut self.tx }
    }

    fn user(&mut self) -> PostgresUserRepository<'_, 'c> {
        PostgresUserRepository { tx: &mut self.tx }
    }

    fn secret_mapping(&mut self) -> PostgresSecretMappingRepository<'_, 'c> {
        PostgresSecretMappingRepository { tx: &mut self.tx }
    }

    fn app(&mut self) -> PostgresAppRepository<'_, 'c> {
        PostgresAppRepository { tx: &mut self.tx }
    }

    async fn commit(self) -> Result<(), DbError> {
        self.tx.commit().await.map_err(map_sqlx)?;
        Ok(())
    }

    async fn rollback(self) -> Result<(), DbError> {
        self.tx.rollback().await.map_err(map_sqlx)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct PostgresDatabase {
    pool: PgPool,
}

impl PostgresDatabase {
    pub async fn connect(url: &str) -> Result<Self, DbError> {
        let opts = PgConnectOptions::from_str(url).map_err(map_sqlx)?;
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect_with(opts)
            .await
            .map_err(map_sqlx)?;
        Ok(Self { pool })
    }
}

impl Database for PostgresDatabase {
    type Tx<'a>
        = PostgresTx<'a>
    where
        Self: 'a;

    async fn begin(&self) -> Result<Self::Tx<'_>, DbError> {
        let tx = self.pool.begin().await.map_err(map_sqlx)?;
        Ok(PostgresTx { tx })
    }
}
