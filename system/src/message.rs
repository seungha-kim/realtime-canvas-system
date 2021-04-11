use super::types::*;
use serde::{Deserialize, Serialize};

// FIXME: 서버 측 ConnectionCommand 가 Debug 를 필요로 함

#[derive(Debug, Serialize, Deserialize)]
pub enum SystemError {
    InvalidSessionId,
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
    Connected { connection_id: ConnectionId },
    JoinedSession { session_id: SessionId },
    LeftSession,
    SessionEvent(SessionEvent),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SessionCommand {
    Fragment(Fragment),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SessionEvent {
    Fragment(Fragment),
}
