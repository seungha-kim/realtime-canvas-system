use crate::message::*;
use crate::traits::{DocumentReadable, PropReadable};

use super::document_storage::*;
use super::transaction_manager::*;
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

    pub fn get_tx(&self, tx_id: &TransactionId) -> Option<&Transaction> {
        self.tx_manager.get(tx_id)
    }

    pub fn from_snapshot(snapshot: DocumentSnapshot) -> Self {
        Self {
            doc_storage: (&snapshot).into(),
            tx_manager: TransactionManager::new(),
        }
    }
}

impl TransactionalStorage {
    pub fn begin(&mut self, tx: Transaction) -> Result<(), ()> {
        self.tx_manager.push(tx.clone());
        // TODO: validation
        Ok(())
    }

    pub fn finish(&mut self, tx_id: &TransactionId, commit: bool) -> Result<Transaction, ()> {
        if let Some(tx) = self.tx_manager.remove(tx_id) {
            if commit {
                // TODO: Err
                self.doc_storage.process(tx.clone()).unwrap();
            }
            Ok(tx)
        } else {
            log::warn!("Tried to finish transaction but doesn't exists: {}", tx_id);
            Err(())
        }
    }
}

impl PropReadable for TransactionalStorage {
    fn get_prop(&self, object_id: &ObjectId, prop_kind: &PropKind) -> Option<&PropValue> {
        let from_kv = self.doc_storage.get_prop(object_id, prop_kind);
        let from_tx = self.tx_manager.get_prop(object_id, prop_kind);
        from_tx.or(from_kv)
    }

    fn get_object_kind(&self, object_id: &ObjectId) -> Option<&ObjectKind> {
        let from_kv = self.doc_storage.get_object_kind(object_id);
        let from_tx = self.tx_manager.get_object_kind(object_id);
        from_tx.or(from_kv)
    }

    fn is_deleted(&self, object_id: &ObjectId) -> Option<bool> {
        self.tx_manager
            .is_deleted(object_id)
            .or(self.doc_storage.is_deleted(object_id))
    }

    fn containing_objects(&self) -> Box<dyn Iterator<Item = &ObjectId> + '_> {
        Box::new(
            self.doc_storage
                .containing_objects()
                .chain(self.tx_manager.containing_objects()),
        )
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
