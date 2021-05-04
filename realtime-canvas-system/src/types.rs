use crate::message::*;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub connections: Vec<ConnectionId>,
}

pub type ObjectId = uuid::Uuid;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PropKey(pub ObjectId, pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectType {
    Document,
    Circle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropName {
    Title,
    PosX,
    PosY,
    Radius,
}

pub type TransactionId = uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
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
