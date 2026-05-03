use actix_web::dev::HttpServiceFactory;
use actix_web::web;
use decodering_db::Database;

use crate::handlers::app::management::create_app;
use crate::handlers::app::management::create_app_user;
use crate::middleware::LockState;

pub fn app_management_routes<D: Database + 'static>() -> impl HttpServiceFactory {
    web::scope(r"/app")
        .wrap(LockState::<D>::new())
        .service(web::resource("/user/create").route(web::post().to(create_app_user::<D>)))
        .route("/create", web::post().to(create_app::<D>))
}
