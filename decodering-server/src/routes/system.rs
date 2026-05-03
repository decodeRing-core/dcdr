use actix_web::Scope;
use actix_web::web;
use decodering_db::Database;

use crate::handlers::system::management::system_init;
use crate::handlers::system::management::system_status;
use crate::handlers::system::management::system_unlock;

pub fn app_system_routes<D: Database + 'static>() -> Scope {
    web::scope(r"/system")
        .service(web::resource("/init").route(web::post().to(system_init::<D>)))
        .service(web::resource("/unlock").route(web::post().to(system_unlock::<D>)))
        .service(web::resource("/status").route(web::get().to(system_status::<D>)))
}
