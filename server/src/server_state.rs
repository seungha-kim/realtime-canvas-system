use crate::session::{Session, SessionBehavior};
use std::collections::HashMap;
use std::num::Wrapping;
use system::{ConnectionId, Document, FileId, SessionId};

pub struct ServerState {
    pub connection_id_source: Wrapping<ConnectionId>,
    pub connection_locations: HashMap<ConnectionId, SessionId>,

    pub session_id_source: Wrapping<SessionId>,
    pub sessions: HashMap<SessionId, Session>,
    pub file_sessions: HashMap<FileId, SessionId>,
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
