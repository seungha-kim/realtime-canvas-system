use actix_web::{web, App, HttpRequest, HttpServer, Responder};
use realtime_canvas_web::server::spawn_server;
use realtime_canvas_web::session::ws_index;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!", &name)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let session_count = Arc::new(AtomicUsize::new(0));

    let srv_tx = spawn_server();

    HttpServer::new(move || {
        App::new()
            .data(session_count.clone())
            .data(srv_tx.clone())
            .route("/ws/", web::get().to(ws_index))
            .route("/{name}", web::get().to(greet))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

/*
ws = new WebSocket('ws://localhost:8080/ws/'); ws.onopen = ws.onmessage = ws.onclose = ws.onerror = console.log;
*/
