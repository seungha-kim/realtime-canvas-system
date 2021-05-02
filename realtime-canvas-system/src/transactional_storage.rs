use crate::message::*;
use crate::traits::ReadableStorage;
use crate::types::*;

use super::document_storage::*;
use super::transaction_manager::*;
use crate::document_command::DocumentCommand;

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

    pub fn handle_command(&mut self) {}

    fn convert_command_to_tx(&self, command: &DocumentCommand) -> Transaction {
        match command {
            DocumentCommand::UpdateDocumentTitle { title } => Transaction {
                items: vec![DocumentMutation::UpdateObject(
                    PropKey(self.doc_storage.document_id(), "title".into()),
                    PropValue::String(title.into()),
                )],
            },
            DocumentCommand::CreateCircle { pos, radius } => {
                let id = uuid::Uuid::new_v4();
                Transaction {
                    items: vec![
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
                            PropValue::Float(*radius),
                        ),
                    ],
                }
            }
            _ => unimplemented!(),
        }
    }

    fn process(&mut self, command_id: CommandId, tx: Transaction) -> Result<(), ()> {
        self.tx_manager.push(command_id, tx);
        Ok(())
    }

    fn ack(&mut self, command_id: CommandId) {
        if let Some(tx) = self.tx_manager.pop(command_id) {
            self.doc_storage.process(tx);
        } else {
            eprintln!("received ack but don't exist: {}", command_id);
        }
    }

    fn nack(&mut self, command_id: CommandId) {
        if self.tx_manager.pop(command_id).is_none() {
            eprintln!("received nack but don't exist: {}", command_id);
        }
    }
}

impl ReadableStorage for TransactionalStorage {
    fn get_string_prop(&self, key: &PropKey) -> Option<&str> {
        let from_kv = self.doc_storage.get_string_prop(key);
        let from_tx = self.tx_manager.get_string_prop(key);
        from_kv.or(from_tx)
    }
}
