use crate::session::{MessageToSession, SenderToSession};
use std::collections::HashMap;

pub type SenderToServer = tokio::sync::mpsc::Sender<MessageToServer>;

#[derive(Debug)]
pub enum MessageToServer {
    Connect { id: usize, tx: SenderToSession },
    Binary(Vec<u8>),
    Disconnect(usize),
}

pub fn spawn_server() -> SenderToServer {
    let (srv_tx, srv_rx) = tokio::sync::mpsc::channel::<MessageToServer>(16);

    tokio::spawn(async move {
        println!("server spawned");
        let mut rx = srv_rx;
        let mut sessions = HashMap::<usize, SenderToSession>::new();
        while let Some(message) = rx.recv().await {
            match message {
                MessageToServer::Connect { id, tx } => {
                    println!("Session connected {}", id);
                    &sessions.insert(id, tx);
                }
                MessageToServer::Binary(vec) => {
                    println!("Session binary message arrived");
                    for (id, ref mut tx) in &mut sessions {
                        println!("Session binary message broadcasting for {}", id);
                        tx.send(MessageToSession::Binary(vec.clone()))
                            .await
                            .unwrap();
                    }
                }
                MessageToServer::Disconnect(id) => {
                    println!("Session disconnected");
                    &mut sessions.remove(&id);
                }
            }
        }
        println!("server terminated");
    });

    return srv_tx;
}
