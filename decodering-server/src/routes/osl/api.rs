use actix_web::web;
use decodering_core::tx::Database;

use crate::handlers::osl::api::api_delete_secret;
use crate::handlers::osl::api::api_destroy_secret;
use crate::handlers::osl::api::api_get_capabilities;
use crate::handlers::osl::api::api_get_secret;
use crate::handlers::osl::api::api_is_tainted_secret;
use crate::handlers::osl::api::api_list_secret;
use crate::handlers::osl::api::api_put_secret;
use crate::handlers::osl::api::api_restore_secret;
use crate::handlers::osl::api::api_taint_secret;
use crate::handlers::osl::api::api_untaint_secret;
use crate::middleware::RaftLeaderHelper;

pub fn read_osl_routes<D: Database + 'static>(cfg: &mut web::ServiceConfig) {
    cfg.route("/secrets/get", web::post().to(api_get_secret::<D>))
        .route("/secrets/list", web::post().to(api_list_secret::<D>))
        .route(
            "/secrets/is-tainted",
            web::post().to(api_is_tainted_secret::<D>),
        )
        .route(
            "/capabilities/get",
            web::get().to(api_get_capabilities::<D>),
        );
}

pub fn write_osl_routes<D: Database + 'static>(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/secrets/put",
        web::post()
            .to(api_put_secret::<D>)
            .wrap(RaftLeaderHelper::<D>::new()),
    )
    .route(
        "/secrets/destroy",
        web::post()
            .to(api_destroy_secret::<D>)
            .wrap(RaftLeaderHelper::<D>::new()),
    )
    .route(
        "/secrets/delete",
        web::post()
            .to(api_delete_secret::<D>)
            .wrap(RaftLeaderHelper::<D>::new()),
    )
    .route(
        "/secrets/restore",
        web::post()
            .to(api_restore_secret::<D>)
            .wrap(RaftLeaderHelper::<D>::new()),
    )
    .route(
        "/secrets/taint",
        web::post()
            .to(api_taint_secret::<D>)
            .wrap(RaftLeaderHelper::<D>::new()),
    )
    .route(
        "/secrets/untaint",
        web::post()
            .to(api_untaint_secret::<D>)
            .wrap(RaftLeaderHelper::<D>::new()),
    );
}
