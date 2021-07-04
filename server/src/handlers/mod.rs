use crate::connection::ws_index;
use crate::handlers::admin::configure_admin_handlers;
use crate::handlers::files::configure_file_handlers;
use actix_web::web;

mod admin;
mod files;

pub fn root(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/ws/{file_id}").route(web::get().to(ws_index)));

    configure_file_handlers(cfg);
    configure_admin_handlers(cfg);
}
