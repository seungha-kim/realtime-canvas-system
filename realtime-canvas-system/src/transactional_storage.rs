use crate::message::*;
use crate::traits::{DocumentReadable, PropReadable};

use super::document_storage::*;
use super::transaction_manager::*;
use crate::document_command::DocumentCommand;
use crate::materialize::InvalidatedMaterial;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub struct TransactionalStorage {
    doc_storage: DocumentStorage,
    tx_manager: TransactionManager,
}

impl TransactionalStorage {
    pub fn new() -> Self {
        Self {
            doc_storage: DocumentStorage::new(),
            tx_manager: TransactionManager::new(),
        }
    }

    pub fn from_snapshot(snapshot: DocumentSnapshot) -> Self {
        Self {
            doc_storage: (&snapshot).into(),
            tx_manager: TransactionManager::new(),
        }
    }
}

impl TransactionalStorage {
    fn tx_manager(&mut self) -> &mut TransactionManager {
        &mut self.tx_manager
    }

    pub fn begin(&mut self, tx: Transaction) -> Result<(), ()> {
        self.tx_manager.push(tx.clone());
        // TODO: validation
        Ok(())
    }

    pub fn finish(&mut self, tx_id: &TransactionId, commit: bool) -> Result<Transaction, ()> {
        if let Some(tx) = self.tx_manager.remove(tx_id) {
            if commit {
                self.doc_storage.process(tx.clone());
            }
            Ok(tx)
        } else {
            log::warn!("Tried to finish transaction but doesn't exists: {}", tx_id);
            Err(())
        }
    }
}

impl PropReadable for TransactionalStorage {
    fn get_string_prop(&self, key: &PropKey) -> Option<&str> {
        let from_kv = self.doc_storage.get_string_prop(key);
        let from_tx = self.tx_manager.get_string_prop(key);
        from_tx.or(from_kv)
    }
}

impl DocumentReadable for TransactionalStorage {
    fn document_id(&self) -> Uuid {
        self.doc_storage.document_id()
    }

    fn snapshot(&self) -> DocumentSnapshot {
        self.doc_storage.snapshot()
    }
}
