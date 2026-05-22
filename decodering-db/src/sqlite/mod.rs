use decodering_core::error::DbError;
use decodering_core::tx::{Database, RaftTx, Tx};
use sqlx::{Sqlite, Transaction};

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

use crate::error::map_sqlx;
use crate::sqlite::api_key::SqliteApiKeysRepository;
use crate::sqlite::app::SqliteAppRepository;
use crate::sqlite::audit::SqliteAuditRepository;
use crate::sqlite::meta::SqliteMetaRepository;
use crate::sqlite::plugin_config::SqlitePluginConfigRepository;
use crate::sqlite::principal::SqlitePrincipalRepository;
use crate::sqlite::principal_app_grant::SqlitePrincipalAppGrantRepository;
use crate::sqlite::principal_credential::SqlitePrincipalCredentialRepository;
use crate::sqlite::principal_token::SqlitePrincipalTokenRepository;
use crate::sqlite::schema::SCHEMA;
use crate::sqlite::secret_mapping::SqliteSecretMappingRepository;
use crate::sqlite::shamir::SqliteShamirRepository;
use crate::sqlite::tpm_challenge::SqliteTpmChallengeRepository;
use crate::sqlite::user::SqliteUserRepository;

mod api_key;
mod app;
mod audit;
mod meta;
mod plugin_config;
mod principal;
mod principal_app_grant;
mod principal_credential;
mod principal_token;
mod schema;
mod secret_mapping;
mod shamir;
mod tpm_challenge;
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
    type PrincipalCredentialRepo<'a>
        = SqlitePrincipalCredentialRepository<'a>
    where
        Self: 'a;
    type PrincipalTokenRepo<'a>
        = SqlitePrincipalTokenRepository<'a>
    where
        Self: 'a;
    type TpmChallengeRepo<'a>
        = SqliteTpmChallengeRepository<'a>
    where
        Self: 'a;
    type PrincipalAppGrantRepo<'a>
        = SqlitePrincipalAppGrantRepository<'a>
    where
        Self: 'a;
    type PluginConfigRepo<'a>
        = SqlitePluginConfigRepository<'a>
    where
        Self: 'a;

    fn tpm_challenge(&mut self) -> SqliteTpmChallengeRepository<'_> {
        SqliteTpmChallengeRepository { tx: &mut self.tx }
    }

    fn principal_app_grant(&mut self) -> SqlitePrincipalAppGrantRepository<'_> {
        SqlitePrincipalAppGrantRepository { tx: &mut self.tx }
    }

    fn principal_token(&mut self) -> SqlitePrincipalTokenRepository<'_> {
        SqlitePrincipalTokenRepository { tx: &mut self.tx }
    }

    fn principal_credential(&mut self) -> SqlitePrincipalCredentialRepository<'_> {
        SqlitePrincipalCredentialRepository { tx: &mut self.tx }
    }

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

    fn plugin_config(&mut self) -> SqlitePluginConfigRepository<'_> {
        SqlitePluginConfigRepository { tx: &mut self.tx }
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
        let opts = SqliteConnectOptions::from_str(url)
            .map_err(map_sqlx)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts)
            .await
            .map_err(map_sqlx)?;

        sqlx::query(SCHEMA).execute(&pool).await.map_err(map_sqlx)?;

        Ok(Self { pool })
    }
}

impl Database for SqliteDatabase {
    type Tx<'a>
        = SqliteTx
    where
        Self: 'a;

    async fn begin(&self) -> Result<Self::Tx<'_>, DbError> {
        let tx = self.pool.begin().await.map_err(map_sqlx)?;
        Ok(SqliteTx { tx })
    }
}
