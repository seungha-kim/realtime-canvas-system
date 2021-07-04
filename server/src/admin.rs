use system::FileId;
use tokio::sync::oneshot::Sender;

#[derive(Debug)]
pub enum AdminCommand {
    GetSessionState { file_id: FileId, tx: Sender<String> },
}
