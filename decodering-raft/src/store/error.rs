use std::fmt;

use decodering_core::error::DbError;

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    RocksDb(rocksdb::Error),
    Db(DbError),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Io(e) => write!(f, "io error: {e}"),
            StorageError::RocksDb(e) => write!(f, "rocksdb error: {e}"),
            StorageError::Db(e) => write!(f, "db error: {e}"),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StorageError::Io(e) => Some(e),
            StorageError::RocksDb(e) => Some(e),
            StorageError::Db(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        StorageError::Io(e)
    }
}

impl From<rocksdb::Error> for StorageError {
    fn from(e: rocksdb::Error) -> Self {
        StorageError::RocksDb(e)
    }
}

impl From<DbError> for StorageError {
    fn from(e: DbError) -> Self {
        StorageError::Db(e)
    }
}
