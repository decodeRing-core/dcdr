#![recursion_limit = "256"]

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    decodering_server::run_with(|_cfg| {}, |_reg| {}).await
}
