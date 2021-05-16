use crate::materialize::Materialize;
use crate::transactional_storage::TransactionalStorage;

use super::message::*;
use crate::document_storage::DocumentSnapshot;
use crate::traits::DocumentReadable;
use uuid::Uuid;

pub struct ServerLeaderDocument {
    storage: TransactionalStorage,
}

impl Materialize<TransactionalStorage> for ServerLeaderDocument {
    fn readable(&self) -> &TransactionalStorage {
        &self.storage
    }
}

impl ServerLeaderDocument {
    pub fn new() -> Self {
        Self {
            storage: TransactionalStorage::new(),
        }
    }

    pub fn process_transaction(&mut self, tx: Transaction) -> Result<Transaction, ()> {
        let tx_id = tx.id;
        // TODO: Err
        self.storage.begin(tx.clone()).unwrap();
        // TODO: validation
        self.storage.finish(&tx_id, true).unwrap();
        Ok(tx)
    }
}

impl DocumentReadable for ServerLeaderDocument {
    fn document_id(&self) -> Uuid {
        unimplemented!()
    }

    fn snapshot(&self) -> DocumentSnapshot {
        self.storage.snapshot()
    }
}
