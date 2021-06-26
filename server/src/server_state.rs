use crate::session::Session;
use std::collections::HashMap;
use std::num::Wrapping;
use system::{ConnectionId, SessionId};

pub enum ConnectionState {
    InLobby,
    Joined(SessionId),
}

pub struct ServerState {
    pub connection_id_source: Wrapping<ConnectionId>,
    pub connection_states: HashMap<ConnectionId, ConnectionState>,

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
            connection_states: HashMap::new(),

            session_id_source: Wrapping(0),
            sessions: HashMap::new(),
        }
    }

    pub fn create_connection(&mut self) -> ConnectionId {
        let connection_id = self.new_connection_id();
        self.connection_states
            .insert(connection_id, ConnectionState::InLobby);

        connection_id
    }

    pub fn create_session(
        &mut self,
        connection_id: &ConnectionId,
    ) -> Result<SessionId, ServerError> {
        if let Some(ConnectionState::InLobby) = self.connection_states.get(&connection_id) {
            let session_id = self.new_session_id();
            self.sessions.insert(session_id, Session::new());
            self.join_session(connection_id, &session_id)
                .map(|_| session_id)
        } else {
            Err(ServerError::InvalidCommandForState)
        }
    }

    pub fn join_session(
        &mut self,
        connection_id: &ConnectionId,
        session_id: &SessionId,
    ) -> Result<(), ServerError> {
        if let Some(ConnectionState::InLobby) = self.connection_states.get(&connection_id) {
            if self
                .sessions
                .get_mut(&session_id)
                .map(|s| s.connections.push(connection_id.clone()))
                .is_none()
            {
                Err(ServerError::InvalidSessionId)
            } else {
                self.connection_states
                    .get_mut(&connection_id)
                    .map(|s| *s = ConnectionState::Joined(session_id.clone()));
                Ok(())
            }
        } else {
            Err(ServerError::InvalidCommandForState)
        }
    }

    pub fn leave_session(&mut self, connection_id: &ConnectionId) -> Option<SessionId> {
        if let Some(ConnectionState::Joined(session_id)) =
            self.connection_states.get(&connection_id)
        {
            let session_id = session_id.clone();
            self.sessions
                .get_mut(&session_id)
                .map(|s| s.connections.retain(|e| e != connection_id));
            self.connection_states
                .get_mut(&connection_id)
                .map(|s| *s = ConnectionState::InLobby);
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

    pub fn disconnect(&mut self, connection_id: &ConnectionId) {
        self.connection_states.remove(&connection_id);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_remove_session_when_all_connections_disconnect() {
        let mut state = ServerState::new();
        let conn = state.create_connection();
        state.create_session(&conn).expect("");
        state.leave_session(&conn).expect("");
        assert!(state.sessions.is_empty())
    }
}
