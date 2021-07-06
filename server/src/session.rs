use system::{ConnectionId, Document, FileId, ServerLeaderDocument, SessionSnapshot};

#[derive(Debug)]
pub struct Session {
    pub file_id: FileId,
    pub connections: Vec<ConnectionId>,
    pub document: ServerLeaderDocument,
    pub behavior: SessionBehavior,
}

#[derive(Debug, Clone)]
pub enum SessionBehavior {
    AutoTerminateWhenEmpty,
    ManualCommitByAdmin,
}

impl Session {
    pub fn new(file_id: FileId, document: Document, behavior: SessionBehavior) -> Self {
        Self {
            file_id,
            connections: Vec::new(),
            document: ServerLeaderDocument::new(document),
            behavior,
        }
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            connections: self.connections.clone(),
        }
    }

    pub fn should_terminate(&self) -> bool {
        match self.behavior {
            SessionBehavior::AutoTerminateWhenEmpty => self.connections.is_empty(),
            _ => false,
        }
    }
}

// TODO
