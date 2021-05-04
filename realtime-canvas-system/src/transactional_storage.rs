use crate::message::*;
use crate::traits::{DocumentReadable, PropReadable};
use crate::types::*;

use super::document_storage::*;
use super::transaction_manager::*;
use crate::document_command::DocumentCommand;
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
}

impl TransactionalStorage {
    fn tx_manager(&mut self) -> &mut TransactionManager {
        &mut self.tx_manager
    }

    pub fn handle_command(&mut self, command: DocumentCommand) {
        let tx = self.convert_command_to_tx(command);
        self.tx_manager.push(tx);
    }

    fn convert_command_to_tx(&self, command: DocumentCommand) -> Transaction {
        match command {
            DocumentCommand::UpdateDocumentTitle { title } => {
                Transaction::new(vec![DocumentMutation::UpdateObject(
                    PropKey(self.doc_storage.document_id(), "title".into()),
                    PropValue::String(title),
                )])
            }
            DocumentCommand::CreateCircle { pos, radius } => {
                let id = uuid::Uuid::new_v4();
                Transaction::new(vec![
                    DocumentMutation::CreateObject(id, ObjectType::Document),
                    DocumentMutation::UpdateObject(
                        PropKey(id, "posX".into()),
                        PropValue::Float(pos.x),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(id, "posY".into()),
                        PropValue::Float(pos.y),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(id, "radius".into()),
                        PropValue::Float(radius),
                    ),
                ])
            }
            _ => unimplemented!(),
        }
    }

    fn process(&mut self, tx: Transaction) -> Result<(), ()> {
        self.tx_manager.push(tx);
        Ok(())
    }

    fn ack(&mut self, tx_id: &TransactionId) {
        if let Some(tx) = self.tx_manager.pop(tx_id) {
            self.doc_storage.process(tx);
        } else {
            eprintln!("received ack but don't exist: {}", tx_id);
        }
    }

    fn nack(&mut self, tx_id: &TransactionId) {
        if self.tx_manager.pop(tx_id).is_none() {
            eprintln!("received nack but don't exist: {}", tx_id);
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
}
