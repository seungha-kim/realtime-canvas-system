use realtime_canvas_system::{ConnectionId, ServerLeaderDocument};

pub struct Session {
    pub connections: Vec<ConnectionId>,
    pub document: ServerLeaderDocument,
}

impl Session {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
            document: ServerLeaderDocument::new(),
        }
    }
}

// TODO
