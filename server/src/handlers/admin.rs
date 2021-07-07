use crate::admin::{AdminCommand, FileDescription};
use crate::document_file::{list_document_files, write_document_file};
use crate::server::{ServerCommand, ServerTx};
use crate::session::SessionBehavior;
use actix_web::error;
use actix_web::web::{self, HttpRequest, HttpResponse};
use actix_web::Responder;
use actix_web::Result;
use askama_actix::Template;
use system::serde::Deserialize;
use system::uuid::Uuid;
use system::{Document, FileId};

#[derive(Template)]
#[template(path = "admin-index.html")]
pub struct AdminConsoleTemplate {
    documents_url: String,
}

pub fn configure_admin_handlers(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .service(web::resource("").route(web::get().to(admin_index)))
            .service(
                web::resource("/documents")
                    .name("admin_documents")
                    .route(web::post().to(create_document))
                    .route(web::get().to(list_documents)),
            )
            .service(
                web::resource("/documents/{file_id}")
                    .name("admin_document")
                    .route(web::get().to(show_document)),
            )
            .service(
                web::resource("/documents/{file_id}/create_manual_session")
                    .name("admin_document_create_manual_session")
                    .route(web::post().to(create_manual_session)),
            ),
    );
}

pub async fn admin_index(req: HttpRequest) -> Result<impl Responder> {
    let documents_url = req
        .url_for("admin_documents", &[""])
        .expect("must match")
        .to_string();
    Ok(AdminConsoleTemplate { documents_url })
}

pub async fn create_document(req: HttpRequest) -> Result<impl Responder> {
    let file_id = Uuid::new_v4();
    let document = Document::new();
    write_document_file(&file_id, &document).await;
    Ok(HttpResponse::Found()
        .header(
            "Location",
            req.url_for("admin_document", &[file_id.to_string()])?
                .into_string(),
        )
        .finish())
}

struct SimpleListItem {
    title: String,
    href: String,
}

#[derive(Template)]
#[template(path = "simple-list.html")]
pub struct SimpleListTemplate {
    items: Vec<SimpleListItem>,
}

pub async fn list_documents(req: HttpRequest) -> Result<impl Responder> {
    let entries = list_document_files().await;
    Ok(SimpleListTemplate {
        items: entries
            .iter()
            .map(|file_id| SimpleListItem {
                title: file_id.to_string(),
                href: req
                    .url_for("admin_document", &[file_id.to_string()])
                    .unwrap()
                    .to_string(),
            })
            .collect(),
    })
}

#[derive(Template)]
#[template(path = "simple-pre.html")]
pub struct SimpleTemplate {
    content: String,
}

#[derive(Template)]
#[template(path = "admin/show-file.html")]
pub struct AdminShowFileTemplate {
    snapshot: String,
    online: bool,
    manual: bool,
}

impl AdminShowFileTemplate {
    fn from_file_description(desc: FileDescription) -> Self {
        let (snapshot, online, manual) = match desc {
            FileDescription::Online(snapshot, SessionBehavior::AutoTerminateWhenEmpty) => {
                (snapshot, true, false)
            }
            FileDescription::Online(snapshot, SessionBehavior::ManualCommitByAdmin) => {
                (snapshot, true, true)
            }
            FileDescription::Offline(snapshot) => (snapshot, false, false),
        };
        Self {
            snapshot,
            online,
            manual,
        }
    }
}

#[derive(Deserialize)]
pub struct ShowDocumentParam {
    file_id: String,
}

pub async fn show_document(
    path: web::Path<ShowDocumentParam>,
    srv_tx: web::Data<ServerTx>,
) -> Result<impl Responder> {
    let file_id = path
        .file_id
        .parse::<FileId>()
        .map_err(|_| error::ErrorBadRequest("invalid format"))?;

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<FileDescription, String>>();

    srv_tx
        .get_ref()
        .clone()
        .send(ServerCommand::AdminCommand(AdminCommand::GetSessionState {
            tx,
            file_id,
        }))
        .await
        .map_err(|_| error::ErrorInternalServerError("Internal Server Error"))?;

    let result = rx
        .await
        .map_err(|_| error::ErrorInternalServerError("Receiver await error"))?;
    let desc = result.map_err(|err| error::ErrorInternalServerError(err))?;

    Ok(AdminShowFileTemplate::from_file_description(desc))
}

#[derive(Deserialize)]
struct CreateManualSessionParam {
    file_id: String,
}

pub async fn create_manual_session(
    path: web::Path<CreateManualSessionParam>,
    srv_tx: web::Data<ServerTx>,
) -> Result<impl Responder> {
    let file_id = path
        .file_id
        .parse::<FileId>()
        .map_err(|_| error::ErrorBadRequest("invalid format"))?;

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<FileDescription, String>>();

    srv_tx
        .get_ref()
        .clone()
        .send(ServerCommand::AdminCommand(AdminCommand::GetSessionState {
            tx,
            file_id,
        }))
        .await
        .map_err(|_| error::ErrorInternalServerError("Internal Server Error"))?;

    let result = rx
        .await
        .map_err(|_| error::ErrorInternalServerError("Receiver await error"))?;
    let desc = result.map_err(|err| error::ErrorInternalServerError(err))?;

    Ok(AdminShowFileTemplate::from_file_description(desc))
}
