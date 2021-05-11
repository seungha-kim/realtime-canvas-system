use realtime_canvas_server::actix_web::{self, web, App, HttpRequest, HttpResponse, HttpServer};
use realtime_canvas_server::connection::ws_index;
use realtime_canvas_server::server::spawn_server;

async fn html_index(_req: HttpRequest) -> HttpResponse {
    let html = include_str!("../../realtime-canvas-wasm/demo/index.html");
    HttpResponse::Ok().content_type("text/html").body(html)
}

async fn app_js(_req: HttpRequest) -> HttpResponse {
    let js = include_str!("../../realtime-canvas-wasm/demo/dist/app.js");
    HttpResponse::Ok().content_type("text/javascript").body(js)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let srv_tx = spawn_server();

    HttpServer::new(move || {
        App::new()
            .data(srv_tx.clone())
            .route("/", web::get().to(html_index))
            .route("/dist/app.js", web::get().to(app_js))
            .route("/ws/", web::get().to(ws_index))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
