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
    pub sessions: HashMap<SessionId, Vec<ConnectionId>>,
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

    pub fn create_session(&mut self, by: &ConnectionId) -> SessionId {
        let session_id = self.new_session_id();
        self.sessions.insert(session_id, Vec::new());
        self.join_session(by, &session_id).unwrap();
        session_id
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
                .map(|v| v.push(connection_id.clone()))
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

    pub fn leave_session(&mut self, connection_id: &ConnectionId) -> Result<(), ()> {
        if let Some(ConnectionState::Joined(session_id)) =
            self.connection_states.get(&connection_id)
        {
            self.sessions
                .get_mut(session_id)
                .map(|v| v.retain(|e| e != connection_id));
            self.connection_states
                .get_mut(&connection_id)
                .map(|s| *s = ConnectionState::InLobby);
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn disconnect(&mut self, connection_id: &ConnectionId) {
        let _ = self.leave_session(connection_id);
        self.connection_states.remove(&connection_id);
    }

    pub fn connection_ids_in_session(
        &self,
        session_id: &SessionId,
    ) -> Result<&[ConnectionId], ServerError> {
        self.sessions
            .get(&session_id)
            .map(|v| v.as_slice())
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
