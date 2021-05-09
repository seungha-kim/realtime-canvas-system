use realtime_canvas_system::{ConnectionId, ServerLeaderDocument, SessionSnapshot};

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

    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            connections: self.connections.clone(),
        }
    }
}

// TODO
