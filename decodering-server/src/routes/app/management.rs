use actix_web::dev::HttpServiceFactory;
use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::app::management::auth_app_user;
use crate::handlers::app::management::create_app;
use crate::handlers::app::management::create_app_user;
use crate::middleware::LockState;

pub fn app_management_routes<D: Database + 'static>() -> impl HttpServiceFactory {
    web::scope(r"/app")
        .wrap(LockState::<D>::new())
        .route("/user/create", web::post().to(create_app_user::<D>))
        .route("/user/auth", web::post().to(auth_app_user::<D>))
        .route("/create", web::post().to(create_app::<D>))
}
