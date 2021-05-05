use tokio::sync::mpsc::{channel, Sender};

use realtime_canvas_system::{
    CommandResult, ConnectionId, FatalError, IdentifiableCommand, IdentifiableEvent,
    SessionCommand, SessionError, SessionEvent, SessionId, SessionState, SystemCommand,
    SystemError, SystemEvent,
};

use super::connection::{ConnectionCommand, ConnectionEvent};
use crate::connection_tx_storage::ConnectionTxStorage;
use crate::server_state::{ConnectionState, ServerState};

pub type ServerTx = Sender<ConnectionCommand>;

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
                let _ = self.leave_session(from).await;
                self.server_state.disconnect(from);
                if let Some(_) = self.connections.remove(from) {
                    self.disconnect(from).await;
                };
            }
            ConnectionCommand::IdentifiableCommand {
                from,
                command:
                    IdentifiableCommand {
                        command_id,
                        system_command,
                    },
            } => match self.handle_system_command(from, system_command).await {
                Ok(system_event) => {
                    self.connections
                        .send(
                            from,
                            ConnectionEvent::IdentifiableEvent(IdentifiableEvent::ByMyself {
                                command_id: command_id.clone(),
                                result: CommandResult::SystemEvent(system_event),
                            }),
                        )
                        .await
                }
                Err(system_error) => match system_error {
                    SystemError::FatalError(ref fatal_error) => {
                        log::warn!(
                            "Disconnecting a connection due to fatal error: {}",
                            fatal_error.reason
                        );
                        self.disconnect(from).await;
                    }
                    system_error => {
                        self.connections
                            .send(
                                from,
                                ConnectionEvent::IdentifiableEvent(IdentifiableEvent::ByMyself {
                                    command_id: command_id.clone(),
                                    result: CommandResult::Error(system_error),
                                }),
                            )
                            .await;
                    }
                },
            },
        }
    }

    async fn handle_system_command(
        &mut self,
        from: &ConnectionId,
        command: &SystemCommand,
    ) -> Result<SystemEvent, SystemError> {
        match command {
            SystemCommand::CreateSession => {
                let session_id = self.server_state.create_session(from);
                let connections = self
                    .server_state
                    .sessions
                    .get(&session_id)
                    .expect("connection vector must exist")
                    .clone();
                Ok(SystemEvent::JoinedSession {
                    session_id,
                    initial_state: SessionState { connections },
                })
            }
            SystemCommand::JoinSession { session_id } => {
                let result = self.server_state.join_session(from, session_id);
                if result.is_ok() {
                    self.broadcast_session_event(
                        session_id,
                        SessionEvent::SomeoneJoined(from.clone()),
                        Some(from),
                    )
                    .await;
                    let connections = self
                        .server_state
                        .sessions
                        .get(&session_id)
                        .expect("connection vector must exists")
                        .clone();
                    Ok(SystemEvent::JoinedSession {
                        session_id: session_id.clone(),
                        initial_state: SessionState { connections },
                    })
                } else {
                    Err(SystemError::InvalidSessionId)
                }
            }
            SystemCommand::LeaveSession => {
                if let Some(_) = self.leave_session(from).await {
                    Ok(SystemEvent::LeftSession)
                } else {
                    Err(SystemError::FatalError(FatalError {
                        reason: "cannot leave session".into(),
                    }))
                }
            }
            SystemCommand::SessionCommand(ref session_command) => self
                .handle_session_command(from, session_command)
                .await
                .map(|v| SystemEvent::SessionEvent(v))
                .map_err(|v| match v {
                    SessionError::FatalError(fatal_error) => SystemError::FatalError(fatal_error),
                }),
        }
    }

    async fn handle_session_command(
        &mut self,
        from: &ConnectionId,
        command: &SessionCommand,
    ) -> Result<SessionEvent, SessionError> {
        if let Some(ConnectionState::Joined(session_id)) =
            self.server_state.connection_states.get(&from)
        {
            let session_id = session_id.clone();
            match command {
                SessionCommand::Fragment(ref fragment) => {
                    let session_event = SessionEvent::Fragment(fragment.clone());
                    self.broadcast_session_event(&session_id, session_event.clone(), Some(from))
                        .await;
                    Ok(session_event)
                }
                _ => unimplemented!(),
            }
        } else {
            Err(SessionError::FatalError(FatalError {
                reason: "connection isn't in any session".into(),
            }))
        }
    }

    async fn broadcast_session_event(
        &mut self,
        session_id: &SessionId,
        session_event: SessionEvent,
        without: Option<&ConnectionId>,
    ) {
        if let Ok(conns) = self.server_state.connection_ids_in_session(session_id) {
            for connection_id in conns {
                if without.map_or(false, |c| c != connection_id) {
                    let event = ConnectionEvent::IdentifiableEvent(IdentifiableEvent::BySystem {
                        system_event: SystemEvent::SessionEvent(session_event.clone()),
                    });
                    self.connections.send(connection_id, event).await;
                }
            }
        } else {
            log::warn!("session has no connection. server state has been corrupted");
        }
    }

    async fn leave_session(&mut self, connection_id: &ConnectionId) -> Option<SessionId> {
        if let Some(session_id) = self.server_state.leave_session(connection_id) {
            self.broadcast_session_event(
                &session_id,
                SessionEvent::SomeoneLeft(connection_id.clone()),
                Some(connection_id),
            )
            .await;
            Some(session_id)
        } else {
            None
        }
    }

    async fn disconnect(&mut self, connection_id: &ConnectionId) {
        // TODO: reason
        self.leave_session(connection_id).await;
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
