use crate::connection::ws_index;
use crate::handlers::admin_console::admin_console_handler;
use crate::handlers::files::handler_files;
use actix_web::web;

mod admin_console;
mod files;

pub fn root(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/")
            .route("/ws/{file_id}/", web::get().to(ws_index))
            .route("/admin/", web::get().to(admin_console_handler))
            .configure(handler_files),
    );
}
