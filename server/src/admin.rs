use crate::session::SessionBehavior;
use system::{FileId, SessionId};
use tokio::sync::oneshot::Sender;

#[derive(Debug)]
pub enum AdminCommand {
    GetSessionState {
        file_id: FileId,
        tx: Sender<Result<FileDescription, String>>,
    },
    OpenManualCommitSession {
        file_id: FileId,
        tx: Sender<Result<SessionId, ()>>,
    },
    CloseManualCommitSession {
        file_id: FileId,
        tx: Sender<Result<(), ()>>,
    },
    CommitManually {
        file_id: FileId,
        tx: Sender<Result<(), ()>>,
    },
}

#[derive(Debug)]
pub enum FileDescription {
    Online {
        debug: String,
        behavior: SessionBehavior,
        has_pending_txs: bool,
    },
    Offline(String),
}
