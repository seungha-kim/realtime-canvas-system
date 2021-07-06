use tokio::sync::mpsc::{channel, Sender};

use system::{
    CommandResult, ConnectionId, DocumentReadable, FileId, IdentifiableCommand, IdentifiableEvent,
    LivePointerEvent, RollbackReason, SessionCommand, SessionError, SessionEvent, SessionId,
};

use super::connection::{ConnectionCommand, ConnectionEvent};
use crate::admin::{AdminCommand, FileDescription};
use crate::connection_tx_storage::ConnectionTxStorage;
use crate::document_file::{read_document_file, write_document_file};
use crate::server_state::ServerState;
use crate::session::SessionBehavior;

pub type ServerTx = Sender<ServerCommand>;

#[derive(Debug)]
pub enum ServerCommand {
    ConnectionCommand(ConnectionCommand),
    AdminCommand(AdminCommand),
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
            ConnectionCommand::Connect { tx, file_id } => {
                let mut tx = tx.clone();

                let (session_id, connection_id) = if self.server_state.session_id(file_id).is_some()
                {
                    self.server_state.join_session(file_id).unwrap()
                } else {
                    let document = read_document_file(file_id).await.unwrap();
                    self.server_state
                        .create_session(file_id, document, SessionBehavior::AutoTerminateWhenEmpty)
                        .unwrap();
                    self.server_state.join_session(file_id).unwrap()
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

    async fn handle_admin_command(&mut self, command: AdminCommand) {
        match command {
            AdminCommand::GetSessionState { file_id, tx } => {
                if let Some(session) = self
                    .server_state
                    .file_sessions
                    .get(&file_id)
                    .and_then(|session_id| self.server_state.sessions.get(session_id))
                {
                    tx.send(Ok(FileDescription::Online(format!("{:#?}", session))))
                        .expect("must success")
                } else {
                    let result = match read_document_file(&file_id).await {
                        Ok(document) => Ok(FileDescription::Offline(format!("{:#?}", document))),
                        Err(_) => Err(format!("No file with id {}", file_id)),
                    };
                    tx.send(result).expect("must success")
                }
            }
            AdminCommand::OpenManualCommitSession { file_id, tx } => {
                let session_id = self
                    .create_session(&file_id, SessionBehavior::ManualCommitByAdmin)
                    .await
                    .unwrap();
                tx.send(Ok(session_id)).unwrap();
            }
            AdminCommand::CloseManualCommitSession { file_id, tx } => {
                if let Some(session_id) = self.server_state.session_id(&file_id).cloned() {
                    self.terminate_session(&session_id).await;
                    tx.send(Ok(())).unwrap();
                }
            }
        };
    }

    async fn create_session(
        &mut self,
        file_id: &FileId,
        behavior: SessionBehavior,
    ) -> Result<SessionId, ()> {
        let document = read_document_file(&file_id).await.unwrap();
        let session_id = self
            .server_state
            .create_session(&file_id, document, behavior)
            .unwrap();

        Ok(session_id)
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

    async fn terminate_session(&mut self, session_id: &SessionId) {
        let session = self.server_state.terminate_session(session_id);
        write_document_file(&session.file_id, session.document.document()).await;
    }

    async fn leave_session(&mut self, connection_id: &ConnectionId) {
        // NOTE: ConnectionCommand::Disconnect 두 번 들어옴
        if let Some(session_id) = self.server_state.leave_session(connection_id) {
            let should_terminate = self
                .server_state
                .get_session(&session_id)
                .expect("must exist")
                .should_terminate();
            if should_terminate {
                self.terminate_session(&session_id).await;
            } else {
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
}

pub fn spawn_server() -> ServerTx {
    let (srv_tx, mut srv_rx) = channel::<ServerCommand>(256);

    tokio::spawn(async move {
        let mut server = Box::new(Server::new());

        while let Some(command) = srv_rx.recv().await {
            match command {
                ServerCommand::ConnectionCommand(connection_command) => {
                    server.handle_connection_command(&connection_command).await;
                }
                ServerCommand::AdminCommand(admin_command) => {
                    server.handle_admin_command(admin_command).await;
                }
            }
        }
    });

    return srv_tx;
}
