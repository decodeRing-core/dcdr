use actix_web::Scope;
use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::system::management::system_init;
use crate::handlers::system::management::system_plugin_config;
use crate::handlers::system::management::system_status;
use crate::handlers::system::management::system_unlock;
use crate::middleware::LockState;

pub fn app_system_routes<D: Database + 'static>() -> Scope {
    web::scope(r"/system")
        .service(web::resource("/init").route(web::post().to(system_init::<D>)))
        .service(
            web::resource("/plugin/config")
                .wrap(LockState::<D>::new())
                .route(web::post().to(system_plugin_config::<D>)),
        )
        .service(web::resource("/unlock").route(web::post().to(system_unlock::<D>)))
        .service(web::resource("/status").route(web::get().to(system_status::<D>)))
}
