use crate::session::Session;
use std::collections::HashMap;
use std::num::Wrapping;
use system::{ConnectionId, SessionId};

pub struct ServerState {
    pub connection_id_source: Wrapping<ConnectionId>,
    pub connection_locations: HashMap<ConnectionId, SessionId>,

    pub session_id_source: Wrapping<SessionId>,
    pub sessions: HashMap<SessionId, Session>,
}

#[derive(Debug)]
pub enum ServerError {
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
        }
    }

    pub fn has_session(&self, session_id: &SessionId) -> bool {
        self.sessions.contains_key(session_id)
    }

    pub fn create_session(
        &mut self,
        session_id: SessionId,
    ) -> Result<(SessionId, ConnectionId), ServerError> {
        // TODO: 파일 개념이 생기고 나면 session_id 을 밖에서 받는 일은 없어야 함
        self.sessions.insert(session_id, Session::new());
        self.join_session(&session_id)
            .map(|connection_id| (session_id, connection_id))
    }

    pub fn join_session(&mut self, session_id: &SessionId) -> Result<ConnectionId, ServerError> {
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
            Ok(connection_id)
        }
    }

    pub fn leave_session(&mut self, connection_id: &ConnectionId) -> Option<SessionId> {
        if let Some(session_id) = self.connection_locations.remove(&connection_id) {
            self.sessions
                .get_mut(&session_id)
                .map(|s| s.connections.retain(|e| e != connection_id));
            self.connection_locations.remove(connection_id);
            if self
                .sessions
                .get(&session_id)
                .map(|s| s.connections.is_empty())
                .unwrap_or(false)
            {
                self.sessions.remove(&session_id);
            }
            Some(session_id)
        } else {
            None
        }
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

    #[allow(dead_code)]
    fn new_session_id(&mut self) -> SessionId {
        self.session_id_source += Wrapping(1);
        self.session_id_source.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_remove_session_when_all_connections_disconnect() {
        let mut state = ServerState::new();
        let (session_id, connection_id) = state.create_session().expect("");
        state.leave_session(&connection_id).expect("");
        assert!(state.sessions.is_empty())
    }
}
