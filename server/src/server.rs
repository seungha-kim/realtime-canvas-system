use tokio::sync::mpsc::{channel, Sender};

use system::{
    CommandResult, ConnectionId, DocumentReadable, IdentifiableCommand, IdentifiableEvent,
    LivePointerEvent, RollbackReason, SessionCommand, SessionError, SessionEvent, SessionId,
};

use super::connection::{ConnectionCommand, ConnectionEvent};
use crate::connection_tx_storage::ConnectionTxStorage;
use crate::server_state::ServerState;

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
            ConnectionCommand::Connect { tx, session_id } => {
                let mut tx = tx.clone();

                let connection_id = if self.server_state.has_session(session_id) {
                    self.server_state.join_session(session_id).unwrap()
                } else {
                    self.server_state
                        .create_session(session_id.clone())
                        .map(|(_, connection_id)| connection_id)
                        .unwrap()
                };

                let session = self
                    .server_state
                    .sessions
                    .get(&session_id)
                    .expect("session must exist");
                let session_snapshot = session.snapshot();
                let document_snapshot = session.document.snapshot();

                if tx
                    .send(ConnectionEvent::IdentifiableEvent(
                        IdentifiableEvent::BySystem {
                            session_event: SessionEvent::Init {
                                session_id: session_id.clone(),
                                session_snapshot: session_snapshot.clone(),
                                document_snapshot,
                            },
                        },
                    ))
                    .await
                    .is_err()
                {
                    return;
                }

                self.connections.insert(connection_id, tx.clone());
                self.connections
                    .send(&connection_id, ConnectionEvent::Connected { connection_id })
                    .await;

                self.broadcast_session_event(
                    &session_id,
                    SessionEvent::SessionStateChanged(session_snapshot),
                    Some(&connection_id),
                )
                .await;
            }
            ConnectionCommand::Disconnect { from } => {
                self.disconnect_from_client(from).await;
            }
            ConnectionCommand::IdentifiableCommand {
                from,
                command:
                    IdentifiableCommand {
                        command_id,
                        session_command,
                    },
            } => match self.handle_session_command(from, session_command).await {
                Ok(session_event) => {
                    if let Some(session_event) = session_event {
                        self.connections
                            .send(
                                from,
                                ConnectionEvent::IdentifiableEvent(IdentifiableEvent::ByMyself {
                                    command_id: command_id.clone(),
                                    result: CommandResult::SessionEvent(session_event),
                                }),
                            )
                            .await
                    }
                }
                Err(session_error) => match session_error {
                    SessionError::FatalError(ref fatal_error) => {
                        log::warn!(
                            "Disconnecting a connection due to fatal error: {}",
                            fatal_error.reason
                        );
                        self.disconnect_from_server(from).await;
                    }
                },
            },
        }
    }

    async fn handle_session_command(
        &mut self,
        from: &ConnectionId,
        command: &SessionCommand,
    ) -> Result<Option<SessionEvent>, SessionError> {
        let session_id = self
            .server_state
            .connection_locations
            .get(from)
            .expect("must be in session")
            .clone();
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
                        session_event: session_event.clone(),
                    });
                    self.connections.send(connection_id, event).await;
                }
            }
        }
    }

    async fn disconnect_from_client(&mut self, connection_id: &ConnectionId) {
        self.leave_session(connection_id).await;
        self.connections.remove(connection_id);
    }

    async fn disconnect_from_server(&mut self, connection_id: &ConnectionId) {
        self.connections
            .send(
                connection_id,
                ConnectionEvent::Disconnected {
                    connection_id: connection_id.clone(),
                },
            )
            .await;
        self.leave_session(connection_id).await;
        self.connections.remove(connection_id);
    }

    async fn leave_session(&mut self, connection_id: &ConnectionId) {
        // NOTE: ConnectionCommand::Disconnect 두 번 들어옴
        if let Some(session_id) = self.server_state.leave_session(connection_id) {
            if let Some(session) = self.server_state.sessions.get(&session_id) {
                let session_snapshot = session.snapshot();
                self.broadcast_session_event(
                    &session_id,
                    SessionEvent::SessionStateChanged(session_snapshot),
                    Some(connection_id),
                )
                .await;
            }
        }
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
