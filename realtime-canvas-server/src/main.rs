use actix_web::{web, App, HttpServer};

use realtime_canvas_server::connection::ws_index;
use realtime_canvas_server::server::spawn_server;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let srv_tx = spawn_server();

    println!("Server started");
    HttpServer::new(move || {
        App::new()
            .data(srv_tx.clone())
            .route("/ws/", web::get().to(ws_index))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
