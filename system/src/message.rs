use super::types::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fragment {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SessionCommand {
    Fragment(Fragment),
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
    JoinedSession { session_id: SessionId },
    LeftSession,
    SessionEvent(SessionEvent),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SessionEvent {
    Fragment(Fragment),
}
