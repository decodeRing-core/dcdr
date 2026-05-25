use std::collections::BTreeMap;

use actix_web::web::Data;
use decodering_core::crypto::decrypt_map;
use decodering_core::repository::PluginConfigRepository;
use decodering_core::tx::{Database, Tx};
use zeroize::Zeroizing;

use crate::app_data::AppData;
use crate::error::ErrorReason;
use crate::handlers::response::ErrorStatus;

pub async fn get_plugin_config_credentials_for_backend<D: Database>(
    db: &mut <D as Database>::Tx<'_>,
    app: &Data<AppData<D>>,
    backend: &str,
) -> Result<BTreeMap<String, Zeroizing<String>>, ErrorStatus> {
    match db.plugin_config().get_by_backend(backend).await {
        Ok(Some(x)) => {
            let Some(key) = app.master_key.get() else {
                tracing::error!("System is locked");
                return Err(ErrorStatus::OperationFailed(ErrorReason::Locked));
            };
            Ok(decrypt_map(key, &x.secret_blob, backend.as_bytes()).unwrap_or_default())
        }
        _ => Ok(BTreeMap::new()),
    }
}
