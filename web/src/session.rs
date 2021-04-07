use crate::server::{MessageToServer, SenderToServer};
use actix::{Actor, ActorContext, AsyncContext, Handler, Message, Running, StreamHandler};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub type SenderToSession = tokio::sync::mpsc::Sender<MessageToSession>;

#[derive(Debug)]
pub enum MessageToSession {
    Binary(Vec<u8>),
}

// CLIENT SESSION

#[derive(Message)]
#[rtype(result = "()")]
struct ClientMessage {
    buf: Vec<u8>,
}

pub struct EchoSession {
    id: usize,
    srv_tx: SenderToServer,
}

impl Actor for EchoSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<MessageToSession>(32);

        self.srv_tx
            .try_send(MessageToServer::Connect { id: self.id, tx })
            .unwrap();

        let addr = ctx.address().recipient();

        tokio::spawn(async move {
            println!("session green thread - started");
            while let Some(message) = rx.recv().await {
                match message {
                    MessageToSession::Binary(vec) => {
                        println!("messageToSession::binary");
                        addr.do_send(ClientMessage { buf: vec }).unwrap();
                    }
                }
            }
            println!("session green thread - terminated");
        });
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // TODO: 버퍼 넘치면 실패함
        self.srv_tx
            .try_send(MessageToServer::Disconnect(self.id))
            .unwrap();
        Running::Stop
    }
}

impl Handler<ClientMessage> for EchoSession {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, ctx: &mut ws::WebsocketContext<Self>) -> Self::Result {
        ctx.binary(msg.buf)
    }
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for EchoSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Binary(bin)) => {
                // TODO: 버퍼 넘치면 실패함
                self.srv_tx
                    .try_send(MessageToServer::Binary(bin.to_vec()))
                    .unwrap();
            }
            Ok(ws::Message::Close(_)) => {
                ctx.stop();
            }
            _ => (),
        }
    }
}

pub async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    srv_tx: web::Data<SenderToServer>,
    cnt: web::Data<Arc<AtomicUsize>>,
) -> Result<HttpResponse, Error> {
    let id = cnt.fetch_add(1, Ordering::SeqCst);
    ws::start(
        EchoSession {
            srv_tx: srv_tx.get_ref().clone(),
            id,
        },
        &req,
        stream,
    )
}
