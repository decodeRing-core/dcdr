use std::str::FromStr;

use decodering_core::error::DbError;
use decodering_core::tx::{Database, Tx};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Postgres, Transaction};

use crate::error::map_sqlx;
use crate::postgres::api_key::PostgresApiKeysRepository;
use crate::postgres::app::PostgresAppRepository;
use crate::postgres::audit::PostgresAuditRepository;
use crate::postgres::auth_challenge::PostgresAuthChallengeRepository;
use crate::postgres::plugin_config::PostgresPluginConfigRepository;
use crate::postgres::principal::PostgresPrincipalRepository;
use crate::postgres::principal_app_grant::PostgresPrincipalAppGrantRepository;
use crate::postgres::principal_credential::PostgresPrincipalCredentialRepository;
use crate::postgres::principal_token::PostgresPrincipalTokenRepository;
use crate::postgres::secret_mapping::PostgresSecretMappingRepository;
use crate::postgres::shamir::PostgresShamirRepository;
use crate::postgres::user::PostgresUserRepository;

mod api_key;
mod app;
mod audit;
mod auth_challenge;
mod plugin_config;
mod principal;
mod principal_app_grant;
mod principal_credential;
mod principal_token;
mod secret_mapping;
mod shamir;
mod user;

pub struct PostgresTx {
    tx: Transaction<'static, Postgres>,
}

impl Tx for PostgresTx {
    type PrincipalRepo<'a>
        = PostgresPrincipalRepository<'a>
    where
        Self: 'a;
    type AuditRepo<'a>
        = PostgresAuditRepository<'a>
    where
        Self: 'a;
    type ShamirRepo<'a>
        = PostgresShamirRepository<'a>
    where
        Self: 'a;
    type ApiKeysRepo<'a>
        = PostgresApiKeysRepository<'a>
    where
        Self: 'a;
    type UserRepo<'a>
        = PostgresUserRepository<'a>
    where
        Self: 'a;
    type AppRepo<'a>
        = PostgresAppRepository<'a>
    where
        Self: 'a;
    type SecretMappingRepo<'a>
        = PostgresSecretMappingRepository<'a>
    where
        Self: 'a;
    type PrincipalCredentialRepo<'a>
        = PostgresPrincipalCredentialRepository<'a>
    where
        Self: 'a;
    type PrincipalTokenRepo<'a>
        = PostgresPrincipalTokenRepository<'a>
    where
        Self: 'a;
    type AuthChallengeRepo<'a>
        = PostgresAuthChallengeRepository<'a>
    where
        Self: 'a;
    type PrincipalAppGrantRepo<'a>
        = PostgresPrincipalAppGrantRepository<'a>
    where
        Self: 'a;
    type PluginConfigRepo<'a>
        = PostgresPluginConfigRepository<'a>
    where
        Self: 'a;

    fn principal_app_grant(&mut self) -> PostgresPrincipalAppGrantRepository<'_> {
        PostgresPrincipalAppGrantRepository { tx: &mut self.tx }
    }

    fn auth_challenge(&mut self) -> PostgresAuthChallengeRepository<'_> {
        PostgresAuthChallengeRepository { tx: &mut self.tx }
    }

    fn principal_token(&mut self) -> PostgresPrincipalTokenRepository<'_> {
        PostgresPrincipalTokenRepository { tx: &mut self.tx }
    }

    fn principal_credential(&mut self) -> PostgresPrincipalCredentialRepository<'_> {
        PostgresPrincipalCredentialRepository { tx: &mut self.tx }
    }

    fn principal(&mut self) -> PostgresPrincipalRepository<'_> {
        PostgresPrincipalRepository { tx: &mut self.tx }
    }

    fn audit(&mut self) -> PostgresAuditRepository<'_> {
        PostgresAuditRepository { tx: &mut self.tx }
    }

    fn shamir(&mut self) -> PostgresShamirRepository<'_> {
        PostgresShamirRepository { tx: &mut self.tx }
    }

    fn api_key(&mut self) -> PostgresApiKeysRepository<'_> {
        PostgresApiKeysRepository { tx: &mut self.tx }
    }

    fn user(&mut self) -> PostgresUserRepository<'_> {
        PostgresUserRepository { tx: &mut self.tx }
    }

    fn secret_mapping(&mut self) -> PostgresSecretMappingRepository<'_> {
        PostgresSecretMappingRepository { tx: &mut self.tx }
    }

    fn app(&mut self) -> PostgresAppRepository<'_> {
        PostgresAppRepository { tx: &mut self.tx }
    }

    fn plugin_config(&mut self) -> PostgresPluginConfigRepository<'_> {
        PostgresPluginConfigRepository { tx: &mut self.tx }
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
        = PostgresTx
    where
        Self: 'a;

    async fn begin(&self) -> Result<Self::Tx<'_>, DbError> {
        let tx = self.pool.begin().await.map_err(map_sqlx)?;
        Ok(PostgresTx { tx })
    }
}
