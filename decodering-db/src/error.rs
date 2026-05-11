use decodering_core::error::DbError;

pub fn map_sqlx(e: sqlx::Error) -> DbError {
    match e {
        sqlx::Error::RowNotFound => DbError::NotFound,
        sqlx::Error::Database(db_err) => {
            let constraint = db_err.constraint().map(str::to_owned);
            let table = db_err.table().map(str::to_owned);
            let msg = db_err.message().to_owned();
            let code = db_err.code().map(std::borrow::Cow::into_owned);

            let detailed = match &table {
                Some(t) => format!("{msg} [table={t}]"),
                None => msg,
            };

            match code.as_deref() {
                Some("2067" | "1555" | "23505") => DbError::UniqueViolation { constraint },
                Some("787" | "23503") => DbError::ForeignKeyViolation { constraint },
                Some("275" | "23514") => DbError::CheckViolation { constraint },
                Some("5" | "6" | "40001" | "40P01") => DbError::SerializationFailure,
                _ => DbError::Other(detailed),
            }
        }
        sqlx::Error::PoolClosed
        | sqlx::Error::PoolTimedOut
        | sqlx::Error::Io(_)
        | sqlx::Error::Tls(_) => DbError::Connection(e.to_string()),
        sqlx::Error::ColumnNotFound(_)
        | sqlx::Error::ColumnDecode { .. }
        | sqlx::Error::Decode(_) => DbError::Serde(e.to_string()),
        sqlx::Error::Migrate(_) => DbError::Schema(e.to_string()),
        other => DbError::Other(other.to_string()),
    }
}
