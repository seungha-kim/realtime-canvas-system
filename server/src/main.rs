use actix_web::{web, App, HttpServer};

use server::admin_console::admin_console_handler;
use server::connection::ws_index;
use server::server::spawn_server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let srv_tx = spawn_server();

    HttpServer::new(move || {
        App::new()
            .data(srv_tx.clone())
            .route("/ws/{session_id}/", web::get().to(ws_index))
            .route("/admin/", web::get().to(admin_console_handler))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
