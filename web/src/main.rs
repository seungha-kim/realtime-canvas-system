use actix::{
    Actor, Addr, AsyncContext, Context, Handler, Message, Recipient, Running, StreamHandler,
};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web_actors::ws;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

async fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!", &name)
}

// SERVER

#[derive(Message)]
#[rtype(result = "()")]
struct EchoMessage {
    buf: Vec<u8>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Connect {
    id: usize,
    addr: Recipient<EchoMessage>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct Disconnect {
    id: usize,
}

struct EchoServer {
    sessions: HashMap<usize, Recipient<EchoMessage>>,
}

impl EchoServer {
    fn new() -> Self {
        EchoServer {
            sessions: HashMap::new(),
        }
    }
}

impl Actor for EchoServer {
    type Context = Context<Self>;
}

impl Handler<Connect> for EchoServer {
    type Result = ();

    fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) -> Self::Result {
        self.sessions.insert(msg.id, msg.addr);
    }
}

impl Handler<Disconnect> for EchoServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, ctx: &mut Context<Self>) -> Self::Result {
        self.sessions.remove(&msg.id);
    }
}

impl Handler<EchoMessage> for EchoServer {
    type Result = ();

    fn handle(&mut self, msg: EchoMessage, ctx: &mut Context<Self>) -> Self::Result {
        for (_, ref addr) in &self.sessions {
            let _ = addr.do_send(EchoMessage {
                buf: msg.buf.clone(),
            });
        }
    }
}

// CLIENT SESSION

#[derive(Message)]
#[rtype(result = "()")]
struct ClientMessage {
    buf: Vec<u8>,
}

struct EchoSession {
    id: usize,
    addr: Addr<EchoServer>,
}

impl Actor for EchoSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.addr.do_send(Connect {
            id: self.id,
            addr: ctx.address().recipient(),
        })
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        self.addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

impl Handler<EchoMessage> for EchoSession {
    type Result = ();

    fn handle(&mut self, msg: EchoMessage, ctx: &mut ws::WebsocketContext<Self>) -> Self::Result {
        ctx.binary(msg.buf)
    }
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for EchoSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Binary(bin)) => self.addr.do_send(EchoMessage { buf: bin.to_vec() }),
            _ => (),
        }
    }
}

async fn index(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<EchoServer>>,
    cnt: web::Data<Arc<AtomicUsize>>,
) -> Result<HttpResponse, Error> {
    let id = cnt.fetch_add(1, Ordering::SeqCst);
    let resp = ws::start(
        EchoSession {
            addr: srv.get_ref().clone(),
            id,
        },
        &req,
        stream,
    );

    println!("{:?}", resp);
    resp
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let session_count = Arc::new(AtomicUsize::new(0));
    let echo_server = EchoServer::new().start();

    HttpServer::new(move || {
        App::new()
            .data(session_count.clone())
            .data(echo_server.clone())
            .route("/ws/", web::get().to(index))
            .route("/{name}", web::get().to(greet))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

/*
ws = new WebSocket('ws://localhost:8080/ws/'); ws.onopen = ws.onmessage = ws.onclose = ws.onerror = console.log;
*/
