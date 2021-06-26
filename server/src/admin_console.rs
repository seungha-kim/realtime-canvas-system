use actix_web::web;
use actix_web::Responder;
use askama_actix::Template;
use system::serde::Deserialize;

#[derive(Template)]
#[template(path = "admin-console.html")]
pub struct AdminConsoleTemplate {
    name: String,
}

#[derive(Deserialize)]
pub struct AdminConsoleQuery {
    name: String,
}

pub async fn admin_console_handler(query: web::Query<AdminConsoleQuery>) -> impl Responder {
    AdminConsoleTemplate {
        name: query.name.to_owned(),
    }
}
