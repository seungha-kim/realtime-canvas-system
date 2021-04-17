use actix::{Actor, ActorContext, AsyncContext, Handler, Message, Running, StreamHandler};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;

use realtime_canvas_system::{bincode, ConnectionId, IdentifiableCommand, IdentifiableEvent};

use crate::connection_tx_storage::ConnectionTx;
use crate::server::ServerTx;

#[derive(Debug)]
pub enum ConnectionCommand {
    Connect {
        tx: ConnectionTx,
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
}

impl Actor for ConnectionActor {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<ConnectionEvent>(32);

        self.srv_tx
            .try_send(ConnectionCommand::Connect { tx })
            .unwrap();

        let addr = ctx.address().recipient();

        tokio::spawn(async move {
            let addr = addr;
            println!("connection green thread - started");
            while let Some(msg) = rx.recv().await {
                addr.try_send(ConnectionActorMessage(msg)).unwrap(); // FIXME: unwrap
            }
            println!("connection green thread - terminated");
        });
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // TODO: 버퍼 넘치면 실패함
        if let ConnectionState::Connected(id) = self.state {
            self.srv_tx
                .try_send(ConnectionCommand::Disconnect { from: id })
                .unwrap();
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
                println!("Ingress size: {}", bin.len());
                // TODO: 버퍼 넘치면 실패함
                if let ConnectionState::Connected(from) = self.state {
                    // TODO: unwrap
                    let command = bincode::deserialize::<IdentifiableCommand>(&bin).unwrap();
                    println!("Ingress {:?}", command);
                    self.srv_tx
                        .try_send(ConnectionCommand::IdentifiableCommand { from, command })
                        .unwrap();
                }
            }
            Ok(ws::Message::Close(_)) => {
                if let ConnectionState::Connected(id) = self.state {
                    self.srv_tx
                        .try_send(ConnectionCommand::Disconnect { from: id })
                        .unwrap();
                    // TODO: realtime-canvas-system 쪽에서 Disconnect 가 처리되었을 때 실제로 커넥션이 끊어지는 메커니즘
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
        println!("Egress {:?}", connection_event);
        match connection_event {
            ConnectionEvent::Connected { connection_id } => {
                self.state = ConnectionState::Connected(*connection_id);
            }
            ConnectionEvent::Disconnected { .. } => {
                // TODO: reason
                ctx.close(None);
            }
            ConnectionEvent::IdentifiableEvent(event) => {
                // TODO: unwrap
                let serialized = bincode::serialize(event).unwrap();
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
    ws::start(
        ConnectionActor {
            srv_tx: srv_tx.get_ref().clone(),
            state: ConnectionState::Idle,
        },
        &req,
        stream,
    )
}
