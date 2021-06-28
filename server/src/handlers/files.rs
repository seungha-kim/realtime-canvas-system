use crate::actix_web::Responder;
use crate::document_file::{list_document_files, write_document_file};
use actix_web::{web, HttpResponse};
use system::serde_json::json;
use system::uuid::Uuid;

pub fn handler_files(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/files")
            .route(web::post().to(post))
            .route(web::get().to(get)),
    );
}

async fn post() -> Result<impl Responder, actix_web::error::Error> {
    let file_id = Uuid::new_v4();
    write_document_file(&file_id).await;
    Ok(HttpResponse::Ok().json(json!({ "fileId": file_id.to_string() })))
}

async fn get() -> Result<impl Responder, actix_web::error::Error> {
    let entries = list_document_files().await;
    Ok(HttpResponse::Ok().json(json!(entries)))
}
