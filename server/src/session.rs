use system::{ConnectionId, Document, FileId, ServerLeaderDocument, SessionSnapshot};

pub struct Session {
    pub file_id: FileId,
    pub connections: Vec<ConnectionId>,
    pub document: ServerLeaderDocument,
}

impl Session {
    pub fn new(file_id: FileId, document: Document) -> Self {
        Self {
            file_id,
            connections: Vec::new(),
            document: ServerLeaderDocument::new(document),
        }
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            connections: self.connections.clone(),
        }
    }
}

// TODO
