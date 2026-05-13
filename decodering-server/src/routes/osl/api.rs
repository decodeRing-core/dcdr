use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::osl::api::{
    api_destroy_secret, api_get_secret, api_list_secret, api_put_secret,
};

pub fn read_osl_routes<D: Database + 'static>(cfg: &mut web::ServiceConfig) {
    cfg.route("/secrets/get", web::post().to(api_get_secret::<D>))
        .route("/secrets/list", web::post().to(api_list_secret::<D>));
}

pub fn write_osl_routes<D: Database + 'static>(cfg: &mut web::ServiceConfig) {
    cfg.route("/secrets/put", web::post().to(api_put_secret::<D>))
        .route("/secrets/destroy", web::post().to(api_destroy_secret::<D>));
}
