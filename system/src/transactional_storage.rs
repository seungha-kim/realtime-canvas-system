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
    fn get_string_prop(&self, key: &PropKey) -> Option<&str> {
        let from_kv = self.doc_storage.get_string_prop(key);
        let from_tx = self.tx_manager.get_string_prop(key);
        from_tx.or(from_kv)
    }

    fn get_id_prop(&self, key: &PropKey) -> Option<&ObjectId> {
        let from_kv = self.doc_storage.get_id_prop(key);
        let from_tx = self.tx_manager.get_id_prop(key);
        from_tx.or(from_kv)
    }

    fn get_float_prop(&self, key: &PropKey) -> Option<&f32> {
        let from_kv = self.doc_storage.get_float_prop(key);
        let from_tx = self.tx_manager.get_float_prop(key);
        from_tx.or(from_kv)
    }

    fn get_color_prop(&self, key: &PropKey) -> Option<&Color> {
        let from_kv = self.doc_storage.get_color_prop(key);
        let from_tx = self.tx_manager.get_color_prop(key);
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
