use std::collections::{HashSet, VecDeque};

use crate::materialize::Materialize;
use crate::transactional_storage::TransactionalStorage;

use super::message::*;
use super::types::*;
use crate::traits::{DocumentReadable, PropReadable};
use crate::DocumentCommand;

pub struct ClientReplicaDocument {
    storage: TransactionalStorage,
}

impl Materialize<TransactionalStorage> for ClientReplicaDocument {
    fn readable(&self) -> &TransactionalStorage {
        &self.storage
    }
}

pub struct TransactionResult {
    pub invalidated_object_ids: HashSet<ObjectId>,
    pub transaction: Transaction,
}

impl ClientReplicaDocument {
    pub fn new() -> Self {
        Self {
            storage: TransactionalStorage::new(),
        }
    }

    pub fn handle_command(&mut self, command: DocumentCommand) -> Result<TransactionResult, ()> {
        log::debug!("Handle document command: {:?}", command);
        let tx = self.convert_command_to_tx(command);
        // TODO: Err
        self.storage.begin(tx.clone());
        Ok(TransactionResult {
            invalidated_object_ids: self.invalidated_object_ids(&tx),
            transaction: tx,
        })
    }

    pub fn handle_transaction(&mut self, tx: Transaction) -> Result<TransactionResult, ()> {
        log::debug!("Handle others transaction: {:?}", tx);
        // TODO: Err
        self.storage.begin(tx.clone());
        self.storage.finish(&tx.id, true);
        Ok(TransactionResult {
            invalidated_object_ids: self.invalidated_object_ids(&tx),
            transaction: tx,
        })
    }

    pub fn handle_ack(&mut self, tx_id: &TransactionId) -> Result<TransactionResult, ()> {
        if let Ok(tx) = self.storage.finish(tx_id, true) {
            Ok(TransactionResult {
                invalidated_object_ids: self.invalidated_object_ids(&tx),
                transaction: tx,
            })
        } else {
            Err(())
        }
    }

    pub fn handle_nack(&mut self, tx_id: &TransactionId) -> Result<TransactionResult, ()> {
        if let Ok(tx) = self.storage.finish(tx_id, false) {
            Ok(TransactionResult {
                invalidated_object_ids: self.invalidated_object_ids(&tx),
                transaction: tx,
            })
        } else {
            Err(())
        }
    }

    fn convert_command_to_tx(&self, command: DocumentCommand) -> Transaction {
        match command {
            DocumentCommand::UpdateDocumentTitle { title } => {
                Transaction::new(vec![DocumentMutation::UpdateObject(
                    PropKey(self.storage.document_id(), "title".into()),
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

    fn invalidated_object_ids(&self, tx: &Transaction) -> HashSet<ObjectId> {
        tx.items
            .iter()
            .map(|m| match m {
                DocumentMutation::CreateObject(object_id, _) => object_id.clone(),
                DocumentMutation::UpdateObject(prop_key, _) => prop_key.0,
                DocumentMutation::DeleteObject(object_id) => object_id.clone(),
            })
            .collect()
    }
}
