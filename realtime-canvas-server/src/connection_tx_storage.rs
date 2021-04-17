use crate::connection::ConnectionEvent;
use realtime_canvas_system::ConnectionId;
use std::collections::HashMap;

pub type ConnectionTx = tokio::sync::mpsc::Sender<ConnectionEvent>;

pub struct ConnectionTxStorage {
    connection_txs: HashMap<ConnectionId, ConnectionTx>,
}

impl ConnectionTxStorage {
    pub fn new() -> Self {
        Self {
            connection_txs: HashMap::new(),
        }
    }

    pub fn insert(&mut self, connection_id: ConnectionId, tx: ConnectionTx) {
        self.connection_txs.insert(connection_id, tx);
    }

    pub async fn send(&mut self, to: &ConnectionId, message: ConnectionEvent) {
        if let Some(tx) = self.connection_txs.get_mut(&to) {
            tx.send(message).await.unwrap(); // FIXME: unwrap
        } else {
            // TODO: WARN
        }
    }

    pub fn remove(&mut self, connection_id: &ConnectionId) -> Option<ConnectionTx> {
        self.connection_txs.remove(connection_id)
    }
}
