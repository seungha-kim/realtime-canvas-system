use actix::{Actor, ActorContext, AsyncContext, Handler, Message, Running, StreamHandler};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;

use system::{bincode, ConnectionId, IdentifiableCommand, IdentifiableEvent, SessionId};

use crate::connection_tx_storage::ConnectionTx;
use crate::server::ServerTx;
use actix_web_actors::ws::{CloseCode, CloseReason};

#[derive(Debug)]
pub enum ConnectionCommand {
    Connect {
        tx: ConnectionTx,
        // TODO: 파일 이름이 나와야 함. session_id 는 내부에서만 다뤄지는 정보여야 함.
        session_id: SessionId,
    },
    Disconnect {
        from: ConnectionId,
    },
    IdentifiableCommand {
        from: ConnectionId,
        command: IdentifiableCommand,
    },
}

#[derive(Debug)]
pub enum ConnectionEvent {
    Connected { connection_id: ConnectionId },
    IdentifiableEvent(IdentifiableEvent),
    Disconnected { connection_id: ConnectionId },
}

#[derive(Message)]
#[rtype(result = "()")]
struct ConnectionActorMessage(ConnectionEvent);

enum ConnectionState {
    Idle,
    Connected(ConnectionId),
}

struct ConnectionActor {
    state: ConnectionState,
    srv_tx: ServerTx,
    session_id: SessionId,
}

impl Actor for ConnectionActor {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ConnectionEvent>(32);

        self.srv_tx
            .try_send(ConnectionCommand::Connect {
                tx,
                session_id: self.session_id,
            })
            .expect("server must not be not closed yet");

        let addr = ctx.address().recipient();

        tokio::spawn(async move {
            let addr = addr;
            log::info!("connection green thread - started");
            while let Some(msg) = rx.recv().await {
                addr.try_send(ConnectionActorMessage(msg))
                    .expect("should have enough buffer")
            }
            log::info!("connection green thread - terminated");
        });
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        if let ConnectionState::Connected(id) = self.state {
            self.srv_tx
                .try_send(ConnectionCommand::Disconnect { from: id })
                .expect("should have enough buffer");
        }

        Running::Stop
    }
}

/// Ingress
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ConnectionActor {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Binary(bin)) => {
                log::debug!("Ingress size: {}", bin.len());
                if let ConnectionState::Connected(from) = self.state {
                    if let Ok(command) = bincode::deserialize::<IdentifiableCommand>(&bin) {
                        log::debug!("Ingress {:?}", command);
                        self.srv_tx
                            .try_send(ConnectionCommand::IdentifiableCommand { from, command })
                            .expect("should have enough buffer");
                    } else {
                        ctx.close(Some(CloseReason {
                            code: CloseCode::Invalid,
                            description: None,
                        }));
                    }
                }
            }
            Ok(ws::Message::Close(_)) => {
                if let ConnectionState::Connected(id) = self.state {
                    self.srv_tx
                        .try_send(ConnectionCommand::Disconnect { from: id })
                        .expect("should have enough buffer");
                    // TODO: system 쪽에서 Disconnect 가 처리되었을 때 실제로 커넥션이 끊어지는 메커니즘
                }
                ctx.stop();
            }
            // TODO: 나머지 유형 확인
            _ => (),
        }
    }
}

/// Egress
impl Handler<ConnectionActorMessage> for ConnectionActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: ConnectionActorMessage,
        ctx: &mut ws::WebsocketContext<Self>,
    ) -> Self::Result {
        let connection_event = &msg.0;
        log::debug!("Egress {:?}", connection_event);
        match connection_event {
            ConnectionEvent::Connected { connection_id } => {
                self.state = ConnectionState::Connected(*connection_id);
            }
            ConnectionEvent::Disconnected { .. } => {
                // TODO: reason
                ctx.close(None);
            }
            ConnectionEvent::IdentifiableEvent(event) => {
                let serialized = bincode::serialize(event).expect("must succeed");
                ctx.binary(serialized);
            }
        }
    }
}

pub async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    srv_tx: web::Data<ServerTx>,
) -> Result<HttpResponse, Error> {
    let session_id: SessionId = req.match_info().get("session_id").unwrap().parse().unwrap();
    ws::start(
        ConnectionActor {
            srv_tx: srv_tx.get_ref().clone(),
            state: ConnectionState::Idle,
            session_id,
        },
        &req,
        stream,
    )
}
