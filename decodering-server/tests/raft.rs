use actix_web::{App, test};
use decodering_db::sqlite::SqliteDatabase;
use decodering_server::routes::config::config_app;

#[actix_web::test]
async fn test_sqlite_init_raft() {
    let app = test::init_service(App::new().configure(config_app::<SqliteDatabase>)).await;

    let req = test::TestRequest::post()
        .uri("/raft/init")
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
}
