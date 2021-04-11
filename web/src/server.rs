use super::connection::{ConnectionCommand, ConnectionEvent};
use std::collections::HashMap;
use std::num::Wrapping;
use system::{
    ConnectionId, IdentifiableCommand, IdentifiableEvent, SessionCommand, SessionEvent, SessionId,
    SystemCommand, SystemEvent,
};
use tokio::sync::mpsc::{channel, Sender};

pub type ServerTx = Sender<ConnectionCommand>;
pub type ConnectionTx = tokio::sync::mpsc::Sender<ConnectionEvent>;

enum ConnectionState {
    InLobby,
    Joined(SessionId),
}

struct Server {
    connection_id_source: Wrapping<ConnectionId>,
    connection_states: HashMap<ConnectionId, ConnectionState>,

    session_id_source: Wrapping<SessionId>,
    sessions: HashMap<SessionId, Vec<ConnectionId>>,
}

impl Server {
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

    fn create_session(&mut self, by: ConnectionId) -> SessionId {
        let session_id = self.new_session_id();
        self.sessions.insert(session_id, Vec::new());
        self.join_session(by, session_id);
        session_id
    }

    fn join_session(&mut self, connection_id: ConnectionId, session_id: SessionId) {
        if let Some(ConnectionState::InLobby) = self.connection_states.get(&connection_id) {
            self.sessions
                .get_mut(&session_id)
                .map(|v| v.push(connection_id));
            self.connection_states
                .get_mut(&connection_id)
                .map(|s| *s = ConnectionState::Joined(session_id));
        }
    }

    fn leave_session(&mut self, connection_id: ConnectionId) {
        if let Some(ConnectionState::Joined(session_id)) =
            self.connection_states.get(&connection_id)
        {
            self.sessions
                .get_mut(session_id)
                .map(|v| v.retain(|e| *e != connection_id));
            self.connection_states
                .get_mut(&connection_id)
                .map(|s| *s = ConnectionState::InLobby);
        }
    }

    fn disconnect(&mut self, connection_id: ConnectionId) {
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

    fn connection_ids_in_session(&self, session_id: SessionId) -> Option<&[ConnectionId]> {
        self.sessions.get(&session_id).map(|v| v.as_slice())
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

    async fn send(&mut self, to: ConnectionId, message: ConnectionEvent) {
        if let Some(tx) = self.connection_txs.get_mut(&to) {
            tx.send(message).await;
        } else {
            // TODO: 연결끊기
        }
    }

    fn remove(&mut self, connection_id: ConnectionId) -> Option<ConnectionTx> {
        self.connection_txs.remove(&connection_id)
    }
}

pub fn spawn_server() -> ServerTx {
    let (srv_tx, mut srv_rx) = channel::<ConnectionCommand>(16);

    tokio::spawn(async move {
        // TIL: 계층 별로 struct 를 나눠야 '파이프라인' 구조가 잘 갖춰지는구나
        //      A 의 정보를 참조해서 (immutable reference) -> B 를 변경하기 (mutable reference)
        // TODO: Box 로 바꾸는게 좋을까?
        let mut server = Server::new();
        let mut connections = ConnectionTxStorage::new();

        while let Some(command) = srv_rx.recv().await {
            match command {
                ConnectionCommand::Connect { tx } => {
                    let connection_id = server.create_connection();
                    connections.insert(connection_id, tx);
                    connections
                        .send(connection_id, ConnectionEvent::Connected { connection_id })
                        .await;
                }
                ConnectionCommand::Disconnect { from } => {
                    server.disconnect(from);
                    if let Some(mut tx) = connections.remove(from) {
                        tx.send(ConnectionEvent::Disconnected {
                            connection_id: from,
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
                } => match system_command {
                    SystemCommand::CreateSession => {
                        let session_id = server.create_session(from);
                        connections
                            .send(
                                from,
                                ConnectionEvent::IdentifiableEvent(IdentifiableEvent::ByMyself {
                                    command_id,
                                    system_event: SystemEvent::JoinedSession { session_id },
                                }),
                            )
                            .await;
                    }
                    SystemCommand::JoinSession { session_id } => {
                        server.join_session(from, session_id);
                        connections
                            .send(
                                from,
                                ConnectionEvent::IdentifiableEvent(IdentifiableEvent::ByMyself {
                                    command_id,
                                    system_event: SystemEvent::JoinedSession { session_id },
                                }),
                            )
                            .await;
                    }
                    SystemCommand::SessionCommand(ref session_command) => {
                        println!("Session binary message arrived");
                        if let Some(ConnectionState::Joined(session_id)) =
                            server.connection_states.get(&from)
                        {
                            match session_command {
                                SessionCommand::Fragment(ref fragment) => {
                                    if let Some(conns) =
                                        server.connection_ids_in_session(*session_id)
                                    {
                                        for connection_id in conns {
                                            let system_event = SystemEvent::SessionEvent(
                                                SessionEvent::Fragment(fragment.clone()),
                                            );
                                            let event = ConnectionEvent::IdentifiableEvent(
                                                if connection_id == &from {
                                                    IdentifiableEvent::ByMyself {
                                                        command_id,
                                                        system_event,
                                                    }
                                                } else {
                                                    IdentifiableEvent::BySystem { system_event }
                                                },
                                            );
                                            connections.send(*connection_id, event).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    SystemCommand::LeaveSession => {
                        server.leave_session(from);
                        connections
                            .send(
                                from,
                                ConnectionEvent::IdentifiableEvent(IdentifiableEvent::ByMyself {
                                    command_id,
                                    system_event: SystemEvent::LeftSession,
                                }),
                            )
                            .await;
                    }
                },
            }
        }
        println!("server terminated");
    });

    return srv_tx;
}
