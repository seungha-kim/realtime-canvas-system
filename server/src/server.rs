use tokio::sync::mpsc::{channel, Sender};

use system::{
    CommandResult, ConnectionId, FatalError, FileId, IdentifiableCommand, IdentifiableEvent,
    LivePointerEvent, RollbackReason, SessionCommand, SessionError, SessionEvent, SessionId,
};

use super::connection::{ConnectionCommand, ConnectionEvent};
use crate::admin::{AdminCommand, FileDescription};
use crate::connection_tx_storage::{ConnectionTx, ConnectionTxStorage};
use crate::document_file::{read_document_file, write_document_file};
use crate::server_state::ServerState;
use crate::session::{
    PendingTransactionCommitError, PendingTransactionCommitResult, SessionBehavior,
};

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

                if self
                    .join_or_create_auto_commit_session(file_id, tx.clone())
                    .await
                    .is_err()
                {
                    // TODO: wrong connection_id
                    tx.send(ConnectionEvent::Disconnected { connection_id: 0 })
                        .await
                        .expect("must succeed");
                }
            }
            ConnectionCommand::Disconnect { from } => {
                self.leave_session(from, true).await;
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
                        self.leave_session(from, true).await;
                        self.disconnect_from_server(from, true).await;
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
                    .get_session_id_of_file(&file_id)
                    .and_then(|session_id| self.server_state.get_session(session_id))
                {
                    tx.send(Ok(FileDescription::Online {
                        debug: format!("{:#?}", session),
                        behavior: session.behavior.clone(),
                        has_pending_txs: session.has_pending_transactions(),
                    }))
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
                match self
                    .create_session(&file_id, SessionBehavior::ManualCommitByAdmin)
                    .await
                {
                    Ok(session_id) => {
                        tx.send(Ok(session_id)).expect("must succeed");
                    }
                    Err(_) => {
                        tx.send(Err(())).expect("must succeed");
                    }
                }
            }
            AdminCommand::CloseManualCommitSession { file_id, tx } => {
                if let Some(session_id) = self.server_state.session_id(&file_id).cloned() {
                    self.terminate_session(&session_id).await;
                    tx.send(Ok(())).expect("must succeed");
                }
            }
            AdminCommand::CommitManually {
                file_id,
                tx: transmit,
            } => {
                let is_valid_command: bool;
                if let Some(session_id) = self.server_state.session_id(&file_id).cloned() {
                    if !self.server_state.has_session(&session_id) {
                        transmit.send(Err(())).expect("must succeed");
                        return;
                    }
                    match self.server_state.commit_pending_transaction(&session_id) {
                        Ok(Some(PendingTransactionCommitResult { tx, from })) => {
                            let ack_event = SessionEvent::TransactionAck(tx.id);
                            self.connections
                                .send(
                                    &from,
                                    ConnectionEvent::IdentifiableEvent(
                                        IdentifiableEvent::BySystem {
                                            session_event: ack_event,
                                        },
                                    ),
                                )
                                .await;

                            self.broadcast_session_event(
                                &session_id,
                                SessionEvent::OthersTransaction(tx),
                                Some(&from),
                            )
                            .await;
                            is_valid_command = true;
                        }
                        Err(err) => match err {
                            PendingTransactionCommitError::InvalidRequest => {
                                is_valid_command = false;
                            }
                            PendingTransactionCommitError::Rollback { from, tx_id } => {
                                let session_event =
                                    SessionEvent::TransactionNack(tx_id, RollbackReason::Something);
                                self.connections
                                    .send(
                                        &from,
                                        ConnectionEvent::IdentifiableEvent(
                                            IdentifiableEvent::BySystem { session_event },
                                        ),
                                    )
                                    .await;
                                is_valid_command = true;
                            }
                        },
                        _ => {
                            panic!("Unexpected branch")
                        }
                    }
                } else {
                    is_valid_command = false;
                }

                if is_valid_command {
                    transmit.send(Ok(())).expect("must succeed");
                } else {
                    transmit.send(Err(())).expect("must succeed");
                }
            }
        };
    }

    async fn handle_session_command(
        &mut self,
        from: &ConnectionId,
        command: &SessionCommand,
    ) -> Result<Option<SessionEvent>, SessionError> {
        let session_id = if let Some(session_id) = self
            .server_state
            .get_session_id_of_connection(from)
            .cloned()
        {
            session_id
        } else {
            return Err(SessionError::FatalError(FatalError {
                reason: "session does not exist".into(),
            }));
        };

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
                    .handle_transaction(&session_id, from, tx.clone());

                match result {
                    Ok(Some(tx)) => {
                        let session_event = SessionEvent::TransactionAck(tx.id.clone());
                        self.broadcast_session_event(
                            &session_id,
                            SessionEvent::OthersTransaction(tx),
                            Some(from),
                        )
                        .await;
                        Ok(Some(session_event))
                    }
                    Ok(None) => Ok(None),
                    Err(_) => Ok(Some(SessionEvent::TransactionNack(
                        tx.id.clone(),
                        RollbackReason::Something,
                    ))),
                }
            }
        }
    }
}

impl Server {
    async fn create_session(
        &mut self,
        file_id: &FileId,
        behavior: SessionBehavior,
    ) -> Result<SessionId, ()> {
        let document = read_document_file(&file_id).await.map_err(|_| ())?;
        let session_id = self
            .server_state
            .create_session(&file_id, document, behavior)
            .map_err(|_| ())?;
        Ok(session_id)
    }

    async fn join_session(
        &mut self,
        file_id: &FileId,
        tx: ConnectionTx,
    ) -> Result<(SessionId, ConnectionId), ()> {
        let (session_id, connection_id) =
            self.server_state.join_session(file_id).map_err(|_| ())?;

        let (session_snapshot, document_snapshot) = self
            .server_state
            .session_initial_snapshot(&session_id)
            .expect("must exist");

        self.connections.insert(connection_id.clone(), tx.clone());
        self.connections
            .send(&connection_id, ConnectionEvent::Connected { connection_id })
            .await;
        self.connections
            .send(
                &connection_id,
                ConnectionEvent::IdentifiableEvent(IdentifiableEvent::BySystem {
                    session_event: SessionEvent::Init {
                        session_id: session_id.clone(),
                        session_snapshot: session_snapshot.clone(),
                        document_snapshot,
                    },
                }),
            )
            .await;
        self.broadcast_session_event(
            &session_id,
            SessionEvent::SessionStateChanged(session_snapshot),
            Some(&connection_id),
        )
        .await;
        Ok((session_id, connection_id))
    }

    async fn join_or_create_auto_commit_session(
        &mut self,
        file_id: &FileId,
        tx: ConnectionTx,
    ) -> Result<(SessionId, ConnectionId), ()> {
        if self.server_state.get_session_id_of_file(file_id).is_none() {
            self.create_session(file_id, SessionBehavior::AutoTerminateWhenEmpty)
                .await
                .expect("must succeed");
        }
        let (session_id, connection_id) = self.join_session(file_id, tx).await?;
        Ok((session_id, connection_id))
    }

    async fn terminate_session(&mut self, session_id: &SessionId) {
        let session = self.server_state.terminate_session(session_id);
        for connection_id in &session.connections {
            self.disconnect_from_server(connection_id, false).await;
        }
        write_document_file(&session.file_id, session.document()).await;
    }

    async fn leave_session(&mut self, connection_id: &ConnectionId, broadcast: bool) {
        // NOTE: ConnectionCommand::Disconnect ??? ??? ?????????
        if let Some(session_id) = self.server_state.leave_session(connection_id) {
            self.connections.remove(connection_id);
            let should_terminate = self
                .server_state
                .get_session(&session_id)
                .map(|s| s.should_terminate())
                .unwrap_or(false);
            if should_terminate {
                self.terminate_session(&session_id).await;
            } else if broadcast {
                self.broadcast_session_state(&session_id).await;
            }
        }
    }

    async fn disconnect_from_server(&mut self, connection_id: &ConnectionId, broadcast: bool) {
        self.connections
            .send(
                connection_id,
                ConnectionEvent::Disconnected {
                    connection_id: connection_id.clone(),
                },
            )
            .await;
        self.connections.remove(connection_id);
        if broadcast {
            if let Some(session_id) = self
                .server_state
                .get_session_id_of_connection(connection_id)
                .cloned()
            {
                self.broadcast_session_state(&session_id).await;
            }
        }
    }
}

impl Server {
    async fn broadcast_session_state(&mut self, session_id: &SessionId) {
        if let Some(session) = self.server_state.get_session(&session_id) {
            let session_snapshot = session.snapshot();
            self.broadcast_session_event(
                &session_id,
                SessionEvent::SessionStateChanged(session_snapshot),
                None,
            )
            .await;
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
