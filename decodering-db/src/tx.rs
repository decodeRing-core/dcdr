use crate::error::DbError;
use crate::repository::ApiKeysRepository;
use crate::repository::AppRepository;
use crate::repository::AuditRepository;
use crate::repository::MetaRepository;
use crate::repository::PrincipalRepository;
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
    type ApiKeysRepo<'a>: ApiKeysRepository
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

    fn principal(&mut self) -> Self::PrincipalRepo<'_>;
    fn audit(&mut self) -> Self::AuditRepo<'_>;
    fn shamir(&mut self) -> Self::ShamirRepo<'_>;
    fn api_key(&mut self) -> Self::ApiKeysRepo<'_>;
    fn user(&mut self) -> Self::UserRepo<'_>;
    fn secret_mapping(&mut self) -> Self::SecretMappingRepo<'_>;
    fn app(&mut self) -> Self::AppRepo<'_>;
    //fn meta(&mut self) -> Self::MetaRepo<'_>;

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
