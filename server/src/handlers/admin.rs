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
use system::{Document, FileId, SessionId};

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
                    .route(web::post().to(open_manual_session)),
            )
            .service(
                web::resource("/documents/{file_id}/close_manual_session")
                    .name("admin_document_close_manual_session")
                    .route(web::post().to(close_manual_session)),
            )
            .service(
                web::resource("/documents/{file_id}/commit_manually")
                    .name("admin_document_commit_manually")
                    .route(web::post().to(commit_manually)),
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
    open_manual_session_action: String,
    close_manual_session_action: String,
    commit_manually_action: String,
    has_pending_txs: bool,
}

impl AdminShowFileTemplate {
    fn from_file_description(req: &HttpRequest, desc: FileDescription, file_id: &FileId) -> Self {
        let (snapshot, online, manual, has_pending_txs) = match desc {
            FileDescription::Online {
                debug,
                behavior: SessionBehavior::AutoTerminateWhenEmpty,
                has_pending_txs,
            } => (debug, true, false, has_pending_txs),
            FileDescription::Online {
                debug,
                behavior: SessionBehavior::ManualCommitByAdmin,
                has_pending_txs,
            } => (debug, true, true, has_pending_txs),
            FileDescription::Offline(snapshot) => (snapshot, false, false, false),
        };
        Self {
            snapshot,
            online,
            manual,
            open_manual_session_action: req
                .url_for(
                    "admin_document_create_manual_session",
                    &[file_id.to_string()],
                )
                .unwrap()
                .to_string(),
            close_manual_session_action: req
                .url_for(
                    "admin_document_close_manual_session",
                    &[file_id.to_string()],
                )
                .unwrap()
                .to_string(),
            commit_manually_action: req
                .url_for("admin_document_commit_manually", &[file_id.to_string()])
                .unwrap()
                .to_string(),
            has_pending_txs,
        }
    }
}

#[derive(Deserialize)]
pub struct ShowDocumentParam {
    file_id: String,
}

pub async fn show_document(
    req: HttpRequest,
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

    Ok(AdminShowFileTemplate::from_file_description(
        &req, desc, &file_id,
    ))
}

#[derive(Deserialize)]
pub struct CreateManualSessionParam {
    file_id: String,
}

pub async fn open_manual_session(
    req: HttpRequest,
    path: web::Path<CreateManualSessionParam>,
    srv_tx: web::Data<ServerTx>,
) -> Result<impl Responder> {
    let file_id = path
        .file_id
        .parse::<FileId>()
        .map_err(|_| error::ErrorBadRequest("invalid format"))?;

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<SessionId, ()>>();

    srv_tx
        .get_ref()
        .clone()
        .send(ServerCommand::AdminCommand(
            AdminCommand::OpenManualCommitSession {
                file_id: file_id.clone(),
                tx,
            },
        ))
        .await
        .map_err(|_| error::ErrorInternalServerError("Internal Server Error"))?;

    let _session_id = rx
        .await
        .map_err(|_| error::ErrorInternalServerError("Receiver await error"))?
        .map_err(|_| error::ErrorInternalServerError("Internal Server Error"))?;

    let redirect_to = req
        .url_for("admin_document", &[file_id.to_string()])
        .map_err(|_| error::ErrorInternalServerError("Internal Server Error"))?
        .to_string();

    Ok(HttpResponse::Found()
        .header("Location", redirect_to)
        .finish())
}

pub async fn close_manual_session(
    req: HttpRequest,
    path: web::Path<CreateManualSessionParam>,
    srv_tx: web::Data<ServerTx>,
) -> Result<impl Responder> {
    let file_id = path
        .file_id
        .parse::<FileId>()
        .map_err(|_| error::ErrorBadRequest("invalid format"))?;

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), ()>>();

    srv_tx
        .get_ref()
        .clone()
        .send(ServerCommand::AdminCommand(
            AdminCommand::CloseManualCommitSession {
                file_id: file_id.clone(),
                tx,
            },
        ))
        .await
        .map_err(|_| error::ErrorInternalServerError("Internal Server Error"))?;

    rx.await
        .map_err(|_| error::ErrorInternalServerError("Receiver await error"))?
        .map_err(|_| error::ErrorInternalServerError("Internal Server Error"))?;

    let redirect_to = req
        .url_for("admin_document", &[file_id.to_string()])
        .map_err(|_| error::ErrorInternalServerError("Internal Server Error"))?
        .to_string();

    Ok(HttpResponse::Found()
        .header("Location", redirect_to)
        .finish())
}

pub async fn commit_manually(
    req: HttpRequest,
    path: web::Path<CreateManualSessionParam>,
    srv_tx: web::Data<ServerTx>,
) -> Result<impl Responder> {
    let file_id = path
        .file_id
        .parse::<FileId>()
        .map_err(|_| error::ErrorBadRequest("invalid format"))?;

    let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), ()>>();

    srv_tx
        .get_ref()
        .clone()
        .send(ServerCommand::AdminCommand(AdminCommand::CommitManually {
            file_id: file_id.clone(),
            tx,
        }))
        .await
        .map_err(|_| error::ErrorInternalServerError("Internal Server Error"))?;

    rx.await
        .map_err(|_| error::ErrorInternalServerError("Receiver await error"))?
        .map_err(|_| error::ErrorInternalServerError("Internal Server Error"))?;

    let redirect_to = req
        .url_for("admin_document", &[file_id.to_string()])
        .map_err(|_| error::ErrorInternalServerError("Internal Server Error"))?
        .to_string();

    Ok(HttpResponse::Found()
        .header("Location", redirect_to)
        .finish())
}
