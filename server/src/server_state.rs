use crate::session::{
    PendingTransactionCommitError, PendingTransactionCommitResult, Session, SessionBehavior,
};
use std::collections::HashMap;
use std::num::Wrapping;
use system::{
    ConnectionId, Document, DocumentSnapshot, FileId, SessionId, SessionSnapshot, Transaction,
};

pub struct ServerState {
    connection_id_source: Wrapping<ConnectionId>,
    connection_locations: HashMap<ConnectionId, SessionId>,

    session_id_source: Wrapping<SessionId>,
    sessions: HashMap<SessionId, Session>,
    file_sessions: HashMap<FileId, SessionId>,
}

#[derive(Debug)]
pub enum ServerError {
    NoSessionForFile,
    SessionAlreadyCreatedForFileId,
    InvalidSessionId,
    InvalidCommandForState,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            connection_id_source: Wrapping(0),
            connection_locations: HashMap::new(),

            session_id_source: Wrapping(0),
            sessions: HashMap::new(),
            file_sessions: HashMap::new(),
        }
    }

    pub fn session_id(&self, file_id: &FileId) -> Option<&SessionId> {
        self.file_sessions.get(file_id)
    }

    pub fn get_session_id_of_connection(&self, connection_id: &ConnectionId) -> Option<&SessionId> {
        self.connection_locations.get(connection_id)
    }

    pub fn get_session_id_of_file(&self, file_id: &FileId) -> Option<&SessionId> {
        self.file_sessions.get(file_id)
    }

    pub fn create_session(
        &mut self,
        file_id: &FileId,
        document: Document,
        behavior: SessionBehavior,
    ) -> Result<SessionId, ServerError> {
        let session_id = self.new_session_id();
        if self.file_sessions.contains_key(file_id) {
            Err(ServerError::SessionAlreadyCreatedForFileId)
        } else {
            self.file_sessions
                .insert(file_id.clone(), session_id.clone());
            self.sessions.insert(
                session_id.clone(),
                Session::new(file_id.clone(), document, behavior),
            );
            Ok(session_id)
        }
    }

    pub fn join_session(
        &mut self,
        file_id: &FileId,
    ) -> Result<(SessionId, ConnectionId), ServerError> {
        if let Some(session_id) = self.file_sessions.get(file_id).cloned() {
            let connection_id = self.new_connection_id();
            if self
                .sessions
                .get_mut(&session_id)
                .map(|s| s.connections.push(connection_id.clone()))
                .is_none()
            {
                Err(ServerError::InvalidSessionId)
            } else {
                self.connection_locations
                    .insert(connection_id.clone(), session_id.clone());
                log::info!("Connection {} joined session {}", connection_id, session_id);
                Ok((session_id, connection_id))
            }
        } else {
            Err(ServerError::NoSessionForFile)
        }
    }

    pub fn leave_session(&mut self, connection_id: &ConnectionId) -> Option<SessionId> {
        if let Some(session_id) = self.connection_locations.remove(&connection_id) {
            self.sessions
                .get_mut(&session_id)
                .map(|s| s.connections.retain(|e| e != connection_id));
            self.connection_locations.remove(connection_id);
            Some(session_id)
        } else {
            None
        }
    }

    pub fn get_session(&self, session_id: &SessionId) -> Option<&Session> {
        self.sessions.get(session_id)
    }

    pub fn connection_ids_in_session(
        &self,
        session_id: &SessionId,
    ) -> Result<&[ConnectionId], ServerError> {
        self.sessions
            .get(&session_id)
            .map(|s| s.connections.as_slice())
            .ok_or(ServerError::InvalidCommandForState)
    }

    fn new_connection_id(&mut self) -> ConnectionId {
        self.connection_id_source += Wrapping(1);
        self.connection_id_source.0
    }

    fn new_session_id(&mut self) -> SessionId {
        self.session_id_source += Wrapping(1);
        self.session_id_source.0
    }

    pub fn terminate_session(&mut self, session_id: &SessionId) -> Session {
        let session = self.sessions.remove(&session_id).expect("must exist");
        self.file_sessions.remove(&session.file_id);
        session
    }

    pub fn session_initial_snapshot(
        &self,
        session_id: &SessionId,
    ) -> Option<(SessionSnapshot, DocumentSnapshot)> {
        let session = self.sessions.get(&session_id)?;
        let session_snapshot = session.snapshot();
        let document_snapshot = session.document_snapshot();
        Some((session_snapshot, document_snapshot))
    }

    pub fn handle_transaction(
        &mut self,
        session_id: &SessionId,
        from: &ConnectionId,
        tx: Transaction,
    ) -> Result<Option<Transaction>, ()> {
        self.sessions
            .get_mut(session_id)
            .expect("must exist")
            .handle_transaction(from, tx)
    }

    pub fn has_session(&mut self, session_id: &SessionId) -> bool {
        self.sessions.contains_key(session_id)
    }

    pub fn commit_pending_transaction(
        &mut self,
        session_id: &SessionId,
    ) -> Result<Option<PendingTransactionCommitResult>, PendingTransactionCommitError> {
        self.sessions
            .get_mut(session_id)
            .expect("must exist")
            .commit_pending_transaction()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use system::uuid::Uuid;

    #[test]
    fn it_remove_session_when_all_connections_disconnect() {
        let mut state = ServerState::new();
        let document = Document::new();
        let (_, connection_id) = state.create_session(&Uuid::new_v4(), document).expect("");
        state.leave_session(&connection_id).expect("");
        assert!(state.sessions.is_empty())
    }
}
