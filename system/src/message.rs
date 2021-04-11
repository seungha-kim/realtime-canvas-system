use super::types::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum IdentifiableEvent {
    ByMyself {
        command_id: CommandId,
        system_event: SystemEvent,
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
