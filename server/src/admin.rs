use system::FileId;
use tokio::sync::oneshot::Sender;

#[derive(Debug)]
pub enum AdminCommand {
    GetSessionState {
        file_id: FileId,
        tx: Sender<Result<FileDescription, String>>,
    },
}

#[derive(Debug)]
pub enum FileDescription {
    Online(String),
    Offline(String),
}
