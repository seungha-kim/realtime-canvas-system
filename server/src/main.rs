use actix_web::{App, HttpServer};
use server::handlers::root;
use server::server::spawn_server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let srv_tx = spawn_server();

    HttpServer::new(move || {
        App::new()
            .wrap(actix_cors::Cors::permissive())
            .data(srv_tx.clone())
            .configure(root)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
