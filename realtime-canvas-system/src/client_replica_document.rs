use std::collections::HashSet;

use crate::materialize::Materialize;
use crate::traits::DocumentReadable;
use crate::transactional_storage::TransactionalStorage;
use crate::{DocumentCommand, DocumentSnapshot};

use super::message::*;

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
    pub fn new(snapshot: DocumentSnapshot) -> Self {
        Self {
            storage: TransactionalStorage::from_snapshot(snapshot),
        }
    }

    pub fn handle_command(&mut self, command: DocumentCommand) -> Result<TransactionResult, ()> {
        log::debug!("Handle document command: {:?}", command);
        let tx = self.convert_command_to_tx(command);
        // TODO: Err
        self.storage.begin(tx.clone()).unwrap();
        Ok(TransactionResult {
            invalidated_object_ids: self.invalidated_object_ids(&tx),
            transaction: tx,
        })
    }

    pub fn handle_transaction(&mut self, tx: Transaction) -> Result<TransactionResult, ()> {
        log::info!("Handle others transaction: {:?}", tx);
        // TODO: Err
        self.storage.begin(tx.clone()).unwrap();
        // TODO: Err
        self.storage.finish(&tx.id, true).unwrap();
        Ok(TransactionResult {
            invalidated_object_ids: self.invalidated_object_ids(&tx),
            transaction: tx,
        })
    }

    pub fn handle_ack(&mut self, tx_id: &TransactionId) -> Result<TransactionResult, ()> {
        log::info!("Ack: {:?}", tx_id);
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
        log::info!("Nack: {:?}", tx_id);
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
            DocumentCommand::UpdateDocumentName { name } => {
                Transaction::new(vec![DocumentMutation::UpdateObject(
                    PropKey(self.storage.document_id(), PropKind::Name),
                    PropValue::String(name),
                )])
            }
            DocumentCommand::CreateOval { pos, r_h, r_v } => {
                let id = uuid::Uuid::new_v4();
                Transaction::new(vec![
                    DocumentMutation::CreateObject(id, ObjectKind::Document),
                    DocumentMutation::UpdateObject(
                        PropKey(id, PropKind::PosX),
                        PropValue::Float(pos.x),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(id, PropKind::PosY),
                        PropValue::Float(pos.y),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(id, PropKind::RadiusH),
                        PropValue::Float(r_h),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(id, PropKind::RadiusV),
                        PropValue::Float(r_v),
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
