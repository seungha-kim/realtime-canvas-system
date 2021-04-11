use super::connection::{ConnectionCommand, ConnectionEvent};
use std::collections::HashMap;
use std::num::Wrapping;
use system::{
    CommandId, CommandResult, ConnectionId, IdentifiableCommand, IdentifiableEvent, SessionCommand,
    SessionEvent, SessionId, SystemCommand, SystemError, SystemEvent,
};
use tokio::sync::mpsc::{channel, Sender};

pub type ServerTx = Sender<ConnectionCommand>;
pub type ConnectionTx = tokio::sync::mpsc::Sender<ConnectionEvent>;

enum ConnectionState {
    InLobby,
    Joined(SessionId),
}

struct ServerState {
    connection_id_source: Wrapping<ConnectionId>,
    connection_states: HashMap<ConnectionId, ConnectionState>,

    session_id_source: Wrapping<SessionId>,
    sessions: HashMap<SessionId, Vec<ConnectionId>>,
}

enum ServerError {
    SystemError(SystemError),
    InvalidCommandForState,
}

impl ServerState {
    fn new() -> Self {
        Self {
            connection_id_source: Wrapping(0),
            connection_states: HashMap::new(),

            session_id_source: Wrapping(0),
            sessions: HashMap::new(),
        }
    }

    fn create_connection(&mut self) -> ConnectionId {
        let connection_id = self.new_connection_id();
        self.connection_states
            .insert(connection_id, ConnectionState::InLobby);

        connection_id
    }

    fn create_session(&mut self, by: &ConnectionId) -> SessionId {
        let session_id = self.new_session_id();
        self.sessions.insert(session_id, Vec::new());
        self.join_session(by, &session_id);
        session_id
    }

    fn join_session(
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
                Err(ServerError::SystemError(SystemError::InvalidSessionId))
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

    fn get_current_session(&self, connection_id: &ConnectionId) -> Option<SessionId> {
        if let Some(ConnectionState::Joined(session_id)) =
            self.connection_states.get(&connection_id)
        {
            Some(session_id.clone())
        } else {
            None
        }
    }

    fn leave_session(&mut self, connection_id: &ConnectionId) -> Result<(), ()> {
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

    fn disconnect(&mut self, connection_id: &ConnectionId) {
        self.leave_session(connection_id);
        self.connection_states.remove(&connection_id);
    }

    fn new_connection_id(&mut self) -> ConnectionId {
        self.connection_id_source += Wrapping(1);
        self.connection_id_source.0
    }

    fn new_session_id(&mut self) -> SessionId {
        self.session_id_source += Wrapping(1);
        self.session_id_source.0
    }

    fn connection_ids_in_session(
        &self,
        session_id: &SessionId,
    ) -> Result<&[ConnectionId], ServerError> {
        self.sessions
            .get(&session_id)
            .map(|v| v.as_slice())
            .ok_or(ServerError::InvalidCommandForState)
    }
}

struct ConnectionTxStorage {
    connection_txs: HashMap<ConnectionId, ConnectionTx>,
}

impl ConnectionTxStorage {
    fn new() -> Self {
        Self {
            connection_txs: HashMap::new(),
        }
    }

    fn insert(&mut self, connection_id: ConnectionId, tx: ConnectionTx) {
        self.connection_txs.insert(connection_id, tx);
    }

    async fn send(&mut self, to: &ConnectionId, message: ConnectionEvent) {
        if let Some(tx) = self.connection_txs.get_mut(&to) {
            tx.send(message).await;
        } else {
            // TODO: WARN
        }
    }

    fn remove(&mut self, connection_id: &ConnectionId) -> Option<ConnectionTx> {
        self.connection_txs.remove(connection_id)
    }
}

struct Server {
    server_state: ServerState,
    connections: ConnectionTxStorage,
}

impl Server {
    fn new() -> Self {
        Self {
            server_state: ServerState::new(),
            connections: ConnectionTxStorage::new(),
        }
    }

    async fn handle_connection_command(&mut self, command: &ConnectionCommand) {
        match command {
            ConnectionCommand::Connect { tx } => {
                let connection_id = self.server_state.create_connection();
                self.connections.insert(connection_id, tx.clone());
                self.connections
                    .send(&connection_id, ConnectionEvent::Connected { connection_id })
                    .await;
            }
            ConnectionCommand::Disconnect { from } => {
                self.server_state.disconnect(from);
                if let Some(mut tx) = self.connections.remove(from) {
                    tx.send(ConnectionEvent::Disconnected {
                        connection_id: from.clone(),
                    })
                    .await;
                };
            }
            ConnectionCommand::IdentifiableCommand {
                from,
                command:
                    IdentifiableCommand {
                        command_id,
                        system_command,
                    },
            } => {
                self.handle_system_command(from, command_id, system_command)
                    .await;
            }
        }
    }

    async fn handle_system_command(
        &mut self,
        from: &ConnectionId,
        command_id: &CommandId,
        command: &SystemCommand,
    ) {
        match command {
            SystemCommand::CreateSession => {
                let session_id = self.server_state.create_session(from);
                self.connections
                    .send(
                        from,
                        ConnectionEvent::IdentifiableEvent(IdentifiableEvent::ByMyself {
                            command_id: command_id.clone(),
                            result: CommandResult::SystemEvent(SystemEvent::JoinedSession {
                                session_id,
                            }),
                        }),
                    )
                    .await;
            }
            SystemCommand::JoinSession { session_id } => {
                let result = self.server_state.join_session(from, session_id);
                self.connections
                    .send(
                        from,
                        ConnectionEvent::IdentifiableEvent(IdentifiableEvent::ByMyself {
                            command_id: command_id.clone(),
                            result: if result.is_ok() {
                                CommandResult::SystemEvent(SystemEvent::JoinedSession {
                                    session_id: session_id.clone(),
                                })
                            } else {
                                CommandResult::Error(SystemError::InvalidSessionId)
                            },
                        }),
                    )
                    .await;
            }
            SystemCommand::LeaveSession => {
                if let Ok(_) = self.server_state.leave_session(from) {
                    self.connections
                        .send(
                            from,
                            ConnectionEvent::IdentifiableEvent(IdentifiableEvent::ByMyself {
                                command_id: command_id.clone(),
                                result: CommandResult::SystemEvent(SystemEvent::LeftSession),
                            }),
                        )
                        .await;
                } else {
                    self.disconnect(from).await;
                }
            }
            SystemCommand::SessionCommand(ref session_command) => {
                self.handle_session_command(from, command_id, session_command)
                    .await;
            }
        }
    }

    async fn handle_session_command(
        &mut self,
        from: &ConnectionId,
        command_id: &CommandId,
        command: &SessionCommand,
    ) {
        if let Some(ConnectionState::Joined(session_id)) =
            self.server_state.connection_states.get(&from)
        {
            match command {
                SessionCommand::Fragment(ref fragment) => {
                    if let Ok(conns) = self.server_state.connection_ids_in_session(session_id) {
                        for connection_id in conns {
                            let system_event =
                                SystemEvent::SessionEvent(SessionEvent::Fragment(fragment.clone()));
                            let event =
                                ConnectionEvent::IdentifiableEvent(if connection_id == from {
                                    IdentifiableEvent::ByMyself {
                                        command_id: command_id.clone(),
                                        result: CommandResult::SystemEvent(system_event),
                                    }
                                } else {
                                    IdentifiableEvent::BySystem { system_event }
                                });
                            self.connections.send(connection_id, event).await;
                        }
                    } else {
                        self.disconnect(from).await;
                    }
                }
            }
        } else {
            self.disconnect(from).await;
        }
    }

    async fn disconnect(&mut self, connection_id: &ConnectionId) {
        // TODO: reason
        self.connections
            .send(
                connection_id,
                ConnectionEvent::Disconnected {
                    connection_id: connection_id.clone(),
                },
            )
            .await;
    }
}

pub fn spawn_server() -> ServerTx {
    let (srv_tx, mut srv_rx) = channel::<ConnectionCommand>(16);

    tokio::spawn(async move {
        let mut server = Box::new(Server::new());

        while let Some(command) = srv_rx.recv().await {
            server.handle_connection_command(&command).await;
        }
    });

    return srv_tx;
}
