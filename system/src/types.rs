use serde::{Deserialize, Serialize};

pub type ConnectionId = u16;
pub type SessionId = u32;
pub type CommandId = u16;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fragment {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}
