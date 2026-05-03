use sqlx::{Sqlite, Transaction};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

use crate::error::DbError;
use crate::sqlite::api_key::SqliteApiKeysRepository;
use crate::sqlite::app::SqliteAppRepository;
use crate::sqlite::audit::SqliteAuditRepository;
use crate::sqlite::meta::SqliteMetaRepository;
use crate::sqlite::principal::SqlitePrincipalRepository;
use crate::sqlite::schema::SCHEMA;
use crate::sqlite::secret_mapping::SqliteSecretMappingRepository;
use crate::sqlite::shamir::SqliteShamirRepository;
use crate::sqlite::user::SqliteUserRepository;
use crate::tx::RaftTx;
use crate::{Database, Tx};

mod api_key;
mod app;
mod audit;
mod meta;
mod principal;
mod schema;
mod secret_mapping;
mod shamir;
mod user;

pub struct SqliteTx {
    tx: Transaction<'static, Sqlite>,
}

impl Tx for SqliteTx {
    type PrincipalRepo<'a>
        = SqlitePrincipalRepository<'a>
    where
        Self: 'a;
    type AuditRepo<'a>
        = SqliteAuditRepository<'a>
    where
        Self: 'a;
    type ShamirRepo<'a>
        = SqliteShamirRepository<'a>
    where
        Self: 'a;
    type ApiKeysRepo<'a>
        = SqliteApiKeysRepository<'a>
    where
        Self: 'a;
    type UserRepo<'a>
        = SqliteUserRepository<'a>
    where
        Self: 'a;
    type AppRepo<'a>
        = SqliteAppRepository<'a>
    where
        Self: 'a;
    type SecretMappingRepo<'a>
        = SqliteSecretMappingRepository<'a>
    where
        Self: 'a;

    fn principal(&mut self) -> SqlitePrincipalRepository<'_> {
        SqlitePrincipalRepository { tx: &mut self.tx }
    }

    fn audit(&mut self) -> SqliteAuditRepository<'_> {
        SqliteAuditRepository { tx: &mut self.tx }
    }

    fn shamir(&mut self) -> SqliteShamirRepository<'_> {
        SqliteShamirRepository { tx: &mut self.tx }
    }

    fn api_key(&mut self) -> SqliteApiKeysRepository<'_> {
        SqliteApiKeysRepository { tx: &mut self.tx }
    }

    fn user(&mut self) -> SqliteUserRepository<'_> {
        SqliteUserRepository { tx: &mut self.tx }
    }

    fn secret_mapping(&mut self) -> SqliteSecretMappingRepository<'_> {
        SqliteSecretMappingRepository { tx: &mut self.tx }
    }

    fn app(&mut self) -> SqliteAppRepository<'_> {
        SqliteAppRepository { tx: &mut self.tx }
    }

    async fn commit(self) -> Result<(), DbError> {
        self.tx.commit().await?;
        Ok(())
    }

    async fn rollback(self) -> Result<(), DbError> {
        self.tx.rollback().await?;
        Ok(())
    }
}

impl RaftTx for SqliteTx {
    type MetaRepo<'a>
        = SqliteMetaRepository<'a>
    where
        Self: 'a;

    fn meta(&mut self) -> SqliteMetaRepository<'_> {
        SqliteMetaRepository { tx: &mut self.tx }
    }
}

#[derive(Clone)]
pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub async fn connect(url: &str) -> Result<Self, DbError> {
        let opts = SqliteConnectOptions::from_str(url)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts)
            .await?;

        sqlx::query(SCHEMA).execute(&pool).await?;

        Ok(Self { pool })
    }
}

impl Database for SqliteDatabase {
    type Tx<'a>
        = SqliteTx
    where
        Self: 'a;

    async fn begin(&self) -> Result<Self::Tx<'_>, DbError> {
        let tx = self.pool.begin().await?;
        Ok(SqliteTx { tx })
    }
}
