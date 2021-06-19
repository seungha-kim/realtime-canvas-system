use super::message::*;
use crate::document_command_transaction::convert_command_to_tx;
use crate::materialize::Materialize;
use crate::traits::DocumentReadable;
use crate::transactional_storage::TransactionalStorage;
use crate::{DocumentCommand, DocumentSnapshot, PropReadable};
use std::collections::HashSet;

pub struct ClientFollowerDocument {
    storage: TransactionalStorage,
}

impl Materialize<TransactionalStorage> for ClientFollowerDocument {
    fn readable(&self) -> &TransactionalStorage {
        &self.storage
    }
}

pub struct TransactionResult {
    pub invalidated_object_ids: HashSet<ObjectId>,
    pub transaction: Transaction,
}

impl ClientFollowerDocument {
    pub fn new(snapshot: DocumentSnapshot) -> Self {
        let storage = TransactionalStorage::from_snapshot(snapshot);
        log::debug!("ClientFollowerDocument created: {}", storage.document_id());
        Self { storage }
    }

    pub fn handle_command(&mut self, command: DocumentCommand) -> Result<TransactionResult, ()> {
        log::debug!("Handle document command: {:?}", command);
        let tx = convert_command_to_tx(&self.storage, command)?;
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

    fn invalidated_object_ids(&self, tx: &Transaction) -> HashSet<ObjectId> {
        let mut result = HashSet::new();
        for m in &tx.items {
            match m {
                DocumentMutation::UpsertProp(
                    PropKey(object_id, PropKind::Parent),
                    Some(PropValue::Reference(parent_id)),
                ) => {
                    if let Some(prev_parent_id) = self
                        .storage
                        .readable_pre_tx()
                        .get_id_prop(&PropKey(object_id.clone(), PropKind::Parent))
                    {
                        result.insert(prev_parent_id.clone());
                    }
                    result.insert(parent_id.clone());
                }
                DocumentMutation::UpsertProp(PropKey(object_id, PropKind::Index), _)
                | DocumentMutation::DeleteObject(object_id) => {
                    let parent_id = self
                        .readable()
                        .get_id_prop(&PropKey(object_id.clone(), PropKind::Parent))
                        .unwrap()
                        .clone();
                    result.insert(parent_id);
                }
                DocumentMutation::UpsertProp(prop_key, _) => {
                    result.insert(prop_key.0);
                }
                _ => {}
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_command_transaction::convert_command_to_tx;
    use crate::DocumentStorage;

    #[test]
    fn it_should_preserve_global_transform_when_changing_parent() {
        let mut doc_storage = DocumentStorage::new();

        let document_id = doc_storage.document_id();
        let frame_id = uuid::Uuid::new_v4();
        let oval_id = uuid::Uuid::new_v4();

        doc_storage
            .process(Transaction::new(vec![
                // frame
                DocumentMutation::CreateObject(frame_id, ObjectKind::Frame),
                DocumentMutation::UpsertProp(
                    PropKey(frame_id, PropKind::PosX),
                    Some(PropValue::Float(10.0)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(frame_id, PropKind::PosY),
                    Some(PropValue::Float(20.0)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(frame_id, PropKind::Parent),
                    Some(PropValue::Reference(document_id)),
                ),
                // oval
                DocumentMutation::CreateObject(oval_id, ObjectKind::Oval),
                DocumentMutation::UpsertProp(
                    PropKey(oval_id, PropKind::PosX),
                    Some(PropValue::Float(100.0)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(oval_id, PropKind::PosY),
                    Some(PropValue::Float(100.0)),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(oval_id, PropKind::Parent),
                    Some(PropValue::Reference(document_id)),
                ),
            ]))
            .unwrap();
        let snapshot = DocumentSnapshot::from(&doc_storage);
        let doc = ClientFollowerDocument::new(snapshot);

        let tx = convert_command_to_tx(
            &doc.storage,
            DocumentCommand::UpdateParent {
                id: oval_id,
                parent_id: frame_id,
            },
        )
        .unwrap();

        let pos_x_after = tx
            .items
            .iter()
            .find_map(|m| match m {
                DocumentMutation::UpsertProp(
                    PropKey(object_id, PropKind::PosX),
                    Some(PropValue::Float(pos_x)),
                ) if object_id == &oval_id => Some(pos_x.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(pos_x_after, 90.0);

        let pos_y_after = tx
            .items
            .iter()
            .find_map(|m| match m {
                DocumentMutation::UpsertProp(
                    PropKey(object_id, PropKind::PosY),
                    Some(PropValue::Float(pos_y)),
                ) if object_id == &oval_id => Some(pos_y.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(pos_y_after, 80.0);
    }
}
