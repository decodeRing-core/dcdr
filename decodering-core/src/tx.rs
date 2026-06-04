use crate::error::DbError;
use crate::repository::ApiKeyRepository;
use crate::repository::AppRepository;
use crate::repository::AuditRepository;
use crate::repository::AuthChallengeRepository;
use crate::repository::MetaRepository;
use crate::repository::PluginConfigRepository;
use crate::repository::PrincipalAppGrantRepository;
use crate::repository::PrincipalCredentialRepository;
use crate::repository::PrincipalRepository;
use crate::repository::PrincipalTokenRepository;
use crate::repository::SecretMappingRespository;
use crate::repository::ShamirRepository;
use crate::repository::UserRepository;

pub trait Tx: Send {
    type PrincipalRepo<'a>: PrincipalRepository
    where
        Self: 'a;
    type AuditRepo<'a>: AuditRepository
    where
        Self: 'a;
    type ShamirRepo<'a>: ShamirRepository
    where
        Self: 'a;
    type ApiKeysRepo<'a>: ApiKeyRepository
    where
        Self: 'a;
    type UserRepo<'a>: UserRepository
    where
        Self: 'a;
    type AppRepo<'a>: AppRepository
    where
        Self: 'a;
    type SecretMappingRepo<'a>: SecretMappingRespository
    where
        Self: 'a;
    type PrincipalCredentialRepo<'a>: PrincipalCredentialRepository
    where
        Self: 'a;
    type PrincipalTokenRepo<'a>: PrincipalTokenRepository
    where
        Self: 'a;
    type AuthChallengeRepo<'a>: AuthChallengeRepository
    where
        Self: 'a;
    type PrincipalAppGrantRepo<'a>: PrincipalAppGrantRepository
    where
        Self: 'a;
    type PluginConfigRepo<'a>: PluginConfigRepository
    where
        Self: 'a;

    fn auth_challenge(&mut self) -> Self::AuthChallengeRepo<'_>;
    fn principal_app_grant(&mut self) -> Self::PrincipalAppGrantRepo<'_>;
    fn principal_token(&mut self) -> Self::PrincipalTokenRepo<'_>;
    fn principal_credential(&mut self) -> Self::PrincipalCredentialRepo<'_>;
    fn principal(&mut self) -> Self::PrincipalRepo<'_>;
    fn audit(&mut self) -> Self::AuditRepo<'_>;
    fn shamir(&mut self) -> Self::ShamirRepo<'_>;
    fn api_key(&mut self) -> Self::ApiKeysRepo<'_>;
    fn user(&mut self) -> Self::UserRepo<'_>;
    fn secret_mapping(&mut self) -> Self::SecretMappingRepo<'_>;
    fn plugin_config(&mut self) -> Self::PluginConfigRepo<'_>;
    fn app(&mut self) -> Self::AppRepo<'_>;

    fn commit(self) -> impl Future<Output = Result<(), DbError>> + Send;
    fn rollback(self) -> impl Future<Output = Result<(), DbError>> + Send;
}

pub trait RaftTx: Tx {
    type MetaRepo<'a>: MetaRepository
    where
        Self: 'a;
    fn meta(&mut self) -> Self::MetaRepo<'_>;
}

pub trait Database: Send + Sync {
    type Tx<'a>: Tx
    where
        Self: 'a;
    fn begin(&self) -> impl Future<Output = Result<Self::Tx<'_>, DbError>> + Send;
}
