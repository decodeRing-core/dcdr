#![recursion_limit = "256"]

use decodering_server::routes::RouteExtensions;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let route_exts = RouteExtensions::default();
    decodering_server::run_with(route_exts, |_reg| {}).await
}
