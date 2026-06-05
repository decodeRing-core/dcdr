use crate::open_api::build_spec;
use actix_web::web;
use utoipa_swagger_ui::SwaggerUi;

pub fn doc_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", build_spec()));
}
