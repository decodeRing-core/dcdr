// use std::fmt;

// #[derive(Debug)]
// pub(crate) enum StoreError {
//     //Sqlx(sqlx::Error),
//     Serde(serde_json::Error),
// }

// impl fmt::Display for StoreError {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             //StoreError::Sqlx(e) => write!(f, "{e}"),
//             StoreError::Serde(e) => write!(f, "serialization failed: {e}"),
//         }
//     }
// }

// impl std::error::Error for StoreError {
//     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//         match self {
//             //StoreError::Sqlx(e) => Some(e),
//             StoreError::Serde(e) => Some(e),
//         }
//     }
// }

// // impl From<sqlx::Error> for StoreError {
// //     fn from(e: sqlx::Error) -> Self {
// //         StoreError::Sqlx(e)
// //     }
// // }

// impl From<serde_json::Error> for StoreError {
//     fn from(e: serde_json::Error) -> Self {
//         StoreError::Serde(e)
//     }
// }
