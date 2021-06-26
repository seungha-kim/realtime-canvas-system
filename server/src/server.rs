use tokio::sync::mpsc::{channel, Sender};

use system::{
    CommandResult, ConnectionId, DocumentReadable, FatalError, IdentifiableCommand,
    IdentifiableEvent, LivePointerEvent, RollbackReason, SessionCommand, SessionError,
    SessionEvent, SessionId, SessionSnapshot, SystemCommand, SystemError, SystemEvent,
};

use super::connection::{ConnectionCommand, ConnectionEvent};
use crate::connection_tx_storage::ConnectionTxStorage;
use crate::server_state::{ConnectionState, ServerError, ServerState};

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
                    if let Some(system_event) = system_event {
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
    ) -> Result<Option<SystemEvent>, SystemError> {
        match command {
            SystemCommand::CreateSession => self
                .server_state
                .create_session(from)
                .and_then(|session_id| {
                    let session = self
                        .server_state
                        .sessions
                        .get(&session_id)
                        .expect("session must exist");
                    let connections = session.connections.clone();
                    Ok(Some(SystemEvent::JoinedSession {
                        session_id,
                        session_snapshot: SessionSnapshot { connections },
                        document_snapshot: session.document.snapshot(),
                    }))
                })
                .map_err(From::from),
            SystemCommand::JoinSession { session_id } => {
                let result = self.server_state.join_session(from, session_id);
                if result.is_ok() {
                    if let Some(session) = self.server_state.sessions.get(&session_id) {
                        let session_snapshot = session.snapshot();
                        let document_snapshot = session.document.snapshot();
                        self.broadcast_session_event(
                            session_id,
                            SessionEvent::SessionStateChanged(session_snapshot.clone()),
                            Some(from),
                        )
                        .await;
                        Ok(Some(SystemEvent::JoinedSession {
                            session_id: session_id.clone(),
                            session_snapshot,
                            document_snapshot,
                        }))
                    } else {
                        log::warn!("Tried to join non-existing session.");
                        Err(SystemError::InvalidSessionId)
                    }
                } else {
                    Err(SystemError::InvalidSessionId)
                }
            }
            SystemCommand::LeaveSession => {
                if let Some(_) = self.leave_session(from).await {
                    Ok(Some(SystemEvent::LeftSession))
                } else {
                    Err(SystemError::FatalError(FatalError {
                        reason: "cannot leave session".into(),
                    }))
                }
            }
            SystemCommand::SessionCommand(ref session_command) => self
                .handle_session_command(from, session_command)
                .await
                .map(|v| v.map(|s| SystemEvent::SessionEvent(s)))
                .map_err(|v| match v {
                    SessionError::FatalError(fatal_error) => SystemError::FatalError(fatal_error),
                }),
        }
    }

    async fn handle_session_command(
        &mut self,
        from: &ConnectionId,
        command: &SessionCommand,
    ) -> Result<Option<SessionEvent>, SessionError> {
        if let Some(ConnectionState::Joined(session_id)) =
            self.server_state.connection_states.get(&from)
        {
            let session_id = session_id.clone();
            match command {
                SessionCommand::LivePointer(live_pointer) => {
                    let session_event = SessionEvent::LivePointer(LivePointerEvent {
                        x: live_pointer.x,
                        y: live_pointer.y,
                        connection_id: from.clone(),
                    });
                    self.broadcast_session_event(&session_id, session_event.clone(), Some(from))
                        .await;
                    Ok(None)
                }
                SessionCommand::Transaction(tx) => {
                    let result = self
                        .server_state
                        .sessions
                        .get_mut(&session_id)
                        .expect("must exist")
                        .document
                        .process_transaction(tx.clone());
                    if let Ok(tx) = result {
                        let session_event = SessionEvent::TransactionAck(tx.id.clone());
                        self.broadcast_session_event(
                            &session_id,
                            SessionEvent::OthersTransaction(tx),
                            Some(from),
                        )
                        .await;
                        Ok(Some(session_event))
                    } else {
                        Ok(Some(SessionEvent::TransactionNack(
                            tx.id.clone(),
                            RollbackReason::Something,
                        )))
                    }
                }
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
        }
    }

    async fn leave_session(&mut self, connection_id: &ConnectionId) -> Option<SessionId> {
        if let Some(ConnectionState::Joined(session_id)) =
            self.server_state.connection_states.get(connection_id)
        {
            let session_id = session_id.clone();
            let session_snapshot = self
                .server_state
                .sessions
                .get(&session_id)
                .expect("session must exist.")
                .snapshot();
            self.server_state.leave_session(connection_id);
            self.broadcast_session_event(
                &session_id,
                SessionEvent::SessionStateChanged(session_snapshot),
                Some(connection_id),
            )
            .await;
            Some(session_id)
        } else {
            log::warn!("connection is not in joined state");
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
    let (srv_tx, mut srv_rx) = channel::<ConnectionCommand>(256);

    tokio::spawn(async move {
        let mut server = Box::new(Server::new());

        while let Some(command) = srv_rx.recv().await {
            server.handle_connection_command(&command).await;
        }
    });

    return srv_tx;
}

impl From<ServerError> for SystemError {
    fn from(e: ServerError) -> Self {
        match e {
            ServerError::InvalidSessionId => SystemError::InvalidSessionId,
            ServerError::InvalidCommandForState => SystemError::InvalidCommandForState,
        }
    }
}
