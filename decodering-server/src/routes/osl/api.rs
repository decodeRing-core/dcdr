use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::osl::api::{api_get_secret, api_put_secret};

pub(crate) fn default_osl_routes<D: Database + 'static>(cfg: &mut web::ServiceConfig) {
    cfg.route("/secrets/put", web::post().to(api_put_secret::<D>))
        .route("/secrets/get", web::post().to(api_get_secret::<D>));
    //.route("/apps/list", web::post().to(api_list_app));
}
