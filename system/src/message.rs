use crate::DocumentSnapshot;
use serde::{Deserialize, Serialize};

pub type ConnectionId = u16;
pub type SessionId = u32;
pub type CommandId = u16;
pub type TransactionId = uuid::Uuid;
pub type ObjectId = uuid::Uuid;

/// FatalError makes connection be closed.
#[derive(Debug, Serialize, Deserialize)]
pub struct FatalError {
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CommandResult {
    SystemEvent(SystemEvent),
    Error(SystemError),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum IdentifiableEvent {
    ByMyself {
        command_id: CommandId,
        result: CommandResult,
    },
    BySystem {
        system_event: SystemEvent,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdentifiableCommand {
    pub command_id: CommandId,
    pub system_command: SystemCommand,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SystemCommand {
    CreateSession,
    JoinSession { session_id: SessionId },
    SessionCommand(SessionCommand),
    LeaveSession,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SystemEvent {
    Connected {
        connection_id: ConnectionId,
    },
    JoinedSession {
        session_id: SessionId,
        session_snapshot: SessionSnapshot,
        document_snapshot: DocumentSnapshot,
    },
    LeftSession,
    SessionEvent(SessionEvent),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SystemError {
    InvalidSessionId,
    FatalError(FatalError),
    SessionError(SessionError),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SessionCommand {
    Fragment(Fragment),
    Transaction(Transaction),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SessionEvent {
    Fragment(Fragment),
    SessionStateChanged(SessionSnapshot),
    SomeoneJoined(ConnectionId),
    SomeoneLeft(ConnectionId),
    TransactionAck(TransactionId),
    TransactionNack(TransactionId, RollbackReason),
    OthersTransaction(Transaction),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SessionError {
    FatalError(FatalError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentMutation {
    CreateObject(ObjectId, ObjectKind),
    UpdateObject(PropKey, PropValue),
    DeleteObject(ObjectId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackReason {
    Something,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fragment {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub connections: Vec<ConnectionId>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum PropKind {
    Parent,
    Name,
    PosX,
    PosY,
    RadiusH,
    RadiusV,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PropKey(pub ObjectId, pub PropKind);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropValue {
    String(String),
    Float(f32),
    Reference(ObjectId),
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub enum ObjectKind {
    Document,
    Frame,
    Oval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: TransactionId,
    pub items: Vec<DocumentMutation>,
}

impl Transaction {
    pub fn new(items: Vec<DocumentMutation>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            items,
        }
    }
}