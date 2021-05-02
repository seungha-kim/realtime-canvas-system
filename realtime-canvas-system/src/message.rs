use super::types::*;
use serde::{Deserialize, Serialize};

// FIXME: 서버 측 ConnectionCommand 가 Debug 를 필요로 함

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
        initial_state: SessionState,
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
    SomeoneJoined(ConnectionId),
    SomeoneLeft(ConnectionId),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SessionError {
    FatalError(FatalError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropValue {
    String(String),
    Float(f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentMutation {
    CreateObject(ObjectId, ObjectType),
    UpdateObject(PropKey, PropValue),
    DeleteObject(ObjectId),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TransactionNackReason {
    Something,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DocumentEvent {
    TransactionAck,
    TransactionNack(TransactionNackReason),
    OthersTransaction(Transaction),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub items: Vec<DocumentMutation>,
}
