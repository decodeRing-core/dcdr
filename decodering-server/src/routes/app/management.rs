use actix_web::dev::HttpServiceFactory;
use actix_web::middleware::from_fn;
use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::app::management::auth_activate_app_user;
use crate::handlers::app::management::auth_app_user;
use crate::handlers::app::management::auth_challenge_app_user;
use crate::handlers::app::management::create_app;
use crate::handlers::app::management::create_app_user;
use crate::handlers::app::management::grant_app_access_user;
use crate::handlers::app::management::list_app_access_user;
use crate::handlers::app::management::revoke_app_access_user;
use crate::middleware::require_raft_initialized;
use crate::middleware::require_raft_leader;
use crate::middleware::require_unlocked;

pub fn app_management_routes<D: Database + 'static>() -> impl HttpServiceFactory {
    web::scope(r"/app")
        .wrap(from_fn(require_unlocked::<D, _>))
        .wrap(from_fn(require_raft_leader::<D, _>))
        .wrap(from_fn(require_raft_initialized::<D, _>))
        .route("/user/grant", web::post().to(grant_app_access_user::<D>))
        .route("/user/revoke", web::post().to(revoke_app_access_user::<D>))
        .route("/user/list", web::post().to(list_app_access_user::<D>))
        .route("/user/create", web::post().to(create_app_user::<D>))
        .route("/user/auth", web::post().to(auth_app_user::<D>))
        .route(
            "/user/auth/challenge",
            web::post().to(auth_challenge_app_user::<D>),
        )
        .route(
            "/user/auth/activate",
            web::post().to(auth_activate_app_user::<D>),
        )
        .route("/create", web::post().to(create_app::<D>))
}
