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
            Self::Io(e) => write!(f, "io error: {e}"),
            Self::RocksDb(e) => write!(f, "rocksdb error: {e}"),
            Self::Db(e) => write!(f, "db error: {e}"),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::RocksDb(e) => Some(e),
            Self::Db(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<rocksdb::Error> for StorageError {
    fn from(e: rocksdb::Error) -> Self {
        Self::RocksDb(e)
    }
}

impl From<DbError> for StorageError {
    fn from(e: DbError) -> Self {
        Self::Db(e)
    }
}
