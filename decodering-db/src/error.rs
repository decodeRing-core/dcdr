use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum DbError {
    NotFound,
    UniqueViolation { constraint: Option<String> },
    ForeignKeyViolation { constraint: Option<String> },
    CheckViolation { constraint: Option<String> },
    SerializationFailure,
    Connection(String),
    Schema(String),
    Serde(String),
    Other(String),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::NotFound => write!(f, "not found"),
            DbError::UniqueViolation {
                constraint: Some(c),
            } => {
                write!(f, "unique constraint violated: {}", c)
            }
            DbError::UniqueViolation { constraint: None } => {
                write!(f, "unique constraint violated")
            }
            DbError::ForeignKeyViolation {
                constraint: Some(c),
            } => {
                write!(f, "foreign key constraint violated: {}", c)
            }
            DbError::ForeignKeyViolation { constraint: None } => {
                write!(f, "foreign key constraint violated")
            }
            DbError::CheckViolation {
                constraint: Some(c),
            } => {
                write!(f, "check constraint violated: {}", c)
            }
            DbError::CheckViolation { constraint: None } => {
                write!(f, "check constraint violated")
            }
            DbError::SerializationFailure => write!(f, "serialization failure; retry"),
            DbError::Connection(msg) => write!(f, "connection error: {}", msg),
            DbError::Schema(msg) => write!(f, "schema error: {}", msg),
            DbError::Serde(msg) => write!(f, "serialization error: {}", msg),
            DbError::Other(msg) => write!(f, "database error: {}", msg),
        }
    }
}

impl From<sqlx::Error> for DbError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => DbError::NotFound,

            sqlx::Error::Database(db_err) => {
                let constraint = db_err.constraint().map(str::to_owned);
                let msg = db_err.message().to_owned();
                let code = db_err.code().map(|c| c.into_owned());

                match code.as_deref() {
                    // SQLite extended codes
                    // SQLITE_CONSTRAINT_UNIQUE = 2067, SQLITE_CONSTRAINT_PRIMARYKEY = 1555
                    Some("2067") | Some("1555") => DbError::UniqueViolation { constraint },
                    // SQLITE_CONSTRAINT_FOREIGNKEY = 787
                    Some("787") => DbError::ForeignKeyViolation { constraint },
                    // SQLITE_CONSTRAINT_CHECK = 275
                    Some("275") => DbError::CheckViolation { constraint },
                    // SQLITE_BUSY = 5, SQLITE_LOCKED = 6 — retryable
                    Some("5") | Some("6") => DbError::SerializationFailure,

                    // Postgres SQLSTATE codes (5-char)
                    // 23505 unique_violation
                    Some("23505") => DbError::UniqueViolation { constraint },
                    // 23503 foreign_key_violation
                    Some("23503") => DbError::ForeignKeyViolation { constraint },
                    // 23514 check_violation
                    Some("23514") => DbError::CheckViolation { constraint },
                    // 40001 serialization_failure, 40P01 deadlock_detected
                    Some("40001") | Some("40P01") => DbError::SerializationFailure,

                    _ => DbError::Other(msg),
                }
            }

            sqlx::Error::PoolClosed | sqlx::Error::PoolTimedOut => {
                DbError::Connection(e.to_string())
            }
            sqlx::Error::Io(_) | sqlx::Error::Tls(_) => DbError::Connection(e.to_string()),

            sqlx::Error::ColumnNotFound(_)
            | sqlx::Error::ColumnDecode { .. }
            | sqlx::Error::Decode(_) => DbError::Serde(e.to_string()),

            sqlx::Error::Migrate(_) => DbError::Schema(e.to_string()),

            other => DbError::Other(other.to_string()),
        }
    }
}

impl Error for DbError {}
