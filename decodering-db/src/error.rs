use decodering_core::error::DbError;

pub fn map_sqlx(e: sqlx::Error) -> DbError {
    match e {
        sqlx::Error::RowNotFound => DbError::NotFound,
        sqlx::Error::Database(db_err) => {
            let constraint = db_err.constraint().map(str::to_owned);
            let msg = db_err.message().to_owned();
            let code = db_err.code().map(|c| c.into_owned());
            match code.as_deref() {
                Some("2067") | Some("1555") => DbError::UniqueViolation { constraint },
                Some("787") => DbError::ForeignKeyViolation { constraint },
                Some("275") => DbError::CheckViolation { constraint },
                Some("5") | Some("6") => DbError::SerializationFailure,
                Some("23505") => DbError::UniqueViolation { constraint },
                Some("23503") => DbError::ForeignKeyViolation { constraint },
                Some("23514") => DbError::CheckViolation { constraint },
                Some("40001") | Some("40P01") => DbError::SerializationFailure,
                _ => DbError::Other(msg),
            }
        }
        sqlx::Error::PoolClosed | sqlx::Error::PoolTimedOut => DbError::Connection(e.to_string()),
        sqlx::Error::Io(_) | sqlx::Error::Tls(_) => DbError::Connection(e.to_string()),
        sqlx::Error::ColumnNotFound(_)
        | sqlx::Error::ColumnDecode { .. }
        | sqlx::Error::Decode(_) => DbError::Serde(e.to_string()),
        sqlx::Error::Migrate(_) => DbError::Schema(e.to_string()),
        other => DbError::Other(other.to_string()),
    }
}
