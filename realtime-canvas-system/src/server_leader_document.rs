use std::collections::{HashSet, VecDeque};

use crate::materialize::Materialize;
use crate::transactional_storage::TransactionalStorage;

use super::message::*;
use crate::traits::{DocumentReadable, PropReadable};
use crate::{DocumentCommand, DocumentSnapshot, DocumentStorage};
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
        self.storage.begin(tx.clone());
        // TODO: validation
        self.storage.finish(&tx_id, true);
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
