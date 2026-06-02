use actix_web::dev::HttpServiceFactory;
use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::app::management::auth_app_user;
use crate::handlers::app::management::auth_aws_app_user;
use crate::handlers::app::management::auth_tpm_app_user;
use crate::handlers::app::management::create_app;
use crate::handlers::app::management::create_app_user;
use crate::handlers::app::management::grant_app_access_user;
use crate::handlers::app::management::list_app_access_user;
use crate::handlers::app::management::revoke_app_access_user;
use crate::handlers::app::management::tpm_activate_app_user;
use crate::handlers::app::management::tpm_challenge_app_user;
use crate::middleware::LockState;
use crate::middleware::RaftInitializedHelper;
use crate::middleware::RaftLeaderHelper;

pub fn app_management_routes<D: Database + 'static>() -> impl HttpServiceFactory {
    web::scope(r"/app")
        .wrap(LockState::<D>::new())
        .wrap(RaftLeaderHelper::<D>::new())
        .wrap(RaftInitializedHelper::<D>::new())
        .route("/user/grant", web::post().to(grant_app_access_user::<D>))
        .route("/user/revoke", web::post().to(revoke_app_access_user::<D>))
        .route("/user/list", web::post().to(list_app_access_user::<D>))
        .route("/user/create", web::post().to(create_app_user::<D>))
        .route("/user/auth", web::post().to(auth_app_user::<D>))
        .route("/user/auth/tpm", web::post().to(auth_tpm_app_user::<D>))
        .route("/user/auth/aws", web::post().to(auth_aws_app_user::<D>))
        .route(
            "/user/tpm/challenge",
            web::post().to(tpm_challenge_app_user::<D>),
        )
        .route(
            "/user/tpm/activate",
            web::post().to(tpm_activate_app_user::<D>),
        )
        .route("/create", web::post().to(create_app::<D>))
}
