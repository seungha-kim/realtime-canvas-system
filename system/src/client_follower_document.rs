use super::message::*;
use crate::document_command_transaction::convert_command_to_tx;
use crate::materialize::Materialize;
use crate::traits::DocumentReadable;
use crate::transactional_storage::TransactionalStorage;
use crate::{DocumentCommand, DocumentSnapshot, PropReadable};
use std::collections::HashSet;

#[derive(Debug)]
pub struct ClientFollowerDocument {
    storage: TransactionalStorage,
    undo_stack: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
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
        Self {
            storage,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn handle_command(&mut self, command: DocumentCommand) -> Result<TransactionResult, ()> {
        log::debug!("Handle document command: {:?}", command);
        let tx = convert_command_to_tx(&self.storage, command)?;

        self.undo_stack.push(tx.inverted(&self.storage));
        self.redo_stack.clear();

        let invalidated_object_ids = self.invalidated_object_ids(&tx);
        self.storage.begin(tx.clone());
        Ok(TransactionResult {
            invalidated_object_ids,
            transaction: tx,
        })
    }

    pub fn handle_transaction(&mut self, tx: Transaction) -> Result<TransactionResult, ()> {
        log::info!("Handle others transaction: {:?}", tx);
        let invalidated_object_ids = self.invalidated_object_ids(&tx);
        self.storage.begin(tx.clone());
        self.storage
            .finish(&tx.id, true)
            .expect("tx from server must be valid");
        Ok(TransactionResult {
            invalidated_object_ids,
            transaction: tx,
        })
    }

    pub fn handle_ack(&mut self, tx_id: &TransactionId) -> Result<TransactionResult, ()> {
        log::info!("Ack: {:?}", tx_id);
        if let Some(tx) = self.storage.get_tx(tx_id) {
            let invalidated_object_ids = self.invalidated_object_ids(&tx);
            Ok(TransactionResult {
                invalidated_object_ids,
                transaction: self.storage.finish(tx_id, true).expect("must finish"),
            })
        } else {
            Err(())
        }
    }

    pub fn handle_nack(&mut self, tx_id: &TransactionId) -> Result<TransactionResult, ()> {
        log::info!("Nack: {:?}", tx_id);
        if let Some(tx) = self.storage.get_tx(tx_id) {
            let invalidated_object_ids = self.invalidated_object_ids(&tx);
            if let Ok(tx) = self.storage.finish(tx_id, false) {
                self.undo_stack.retain(|item| &item.id != tx_id);
                self.redo_stack.retain(|item| &item.id != tx_id);
                Ok(TransactionResult {
                    invalidated_object_ids,
                    transaction: tx,
                })
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    pub fn undo(&mut self) -> Result<TransactionResult, ()> {
        if let Some(tx) = self.undo_stack.pop() {
            let inverted = tx.inverted(self.readable());
            log::info!("creating redo transaction {:?}", inverted);
            self.redo_stack.push(inverted);

            let invalidated_object_ids = self.invalidated_object_ids(&tx);
            self.storage.begin(tx.clone());
            Ok(TransactionResult {
                invalidated_object_ids,
                transaction: tx,
            })
        } else {
            // TODO: Err type
            Err(())
        }
    }

    pub fn redo(&mut self) -> Result<TransactionResult, ()> {
        if let Some(tx) = self.redo_stack.pop() {
            let inverted = tx.inverted(self.readable());
            log::info!("creating undo transaction {:?}", inverted);
            self.undo_stack.push(inverted);

            let invalidated_object_ids = self.invalidated_object_ids(&tx);
            self.storage.begin(tx.clone());
            Ok(TransactionResult {
                invalidated_object_ids,
                transaction: tx,
            })
        } else {
            // TODO: Err type
            Err(())
        }
    }

    /// Returns the object ids to be invalidated by the incoming transaction.
    ///
    /// Should be called right before beginning/finishing transaction.
    fn invalidated_object_ids(&self, tx: &Transaction) -> HashSet<ObjectId> {
        let mut result = HashSet::new();
        for m in &tx.items {
            match m {
                DocumentMutation::UpsertProp(
                    object_id,
                    PropKind::Parent,
                    Some(PropValue::Reference(parent_id)),
                ) => {
                    if let Some(prev_parent_id) =
                        self.readable().get_id_prop(object_id, &PropKind::Parent)
                    {
                        result.insert(prev_parent_id.clone());
                    }
                    result.insert(parent_id.clone());
                }
                DocumentMutation::UpsertProp(object_id, PropKind::Index, _)
                | DocumentMutation::DeleteObject(object_id) => {
                    if let Some(parent_id) =
                        self.readable().get_id_prop(object_id, &PropKind::Parent)
                    {
                        result.insert(parent_id.clone());
                    }
                }
                DocumentMutation::UpsertProp(object_id, ..) => {
                    result.insert(object_id.clone());
                }
                _ => {}
            }
        }
        result
    }
}

impl Transaction {
    pub fn inverted<R: PropReadable + DocumentReadable>(&self, r: &R) -> Transaction {
        let mut mutations = Vec::new();

        for m in &self.items {
            match m {
                DocumentMutation::CreateObject(object_id, _) => {
                    mutations.push(DocumentMutation::DeleteObject(object_id.clone()));
                }
                DocumentMutation::UpsertProp(object_id, prop_kind, _) => {
                    let prev_value = r.get_prop(object_id, prop_kind);
                    mutations.push(DocumentMutation::UpsertProp(
                        object_id.clone(),
                        prop_kind.clone(),
                        prev_value.cloned(),
                    ))
                }
                DocumentMutation::DeleteObject(object_id) => {
                    let object_kind = r.get_object_kind(object_id).expect("must exist");
                    mutations.push(DocumentMutation::CreateObject(
                        object_id.clone(),
                        object_kind.clone(),
                    ))
                }
            }
        }
        mutations.reverse();

        Self {
            id: self.id,
            items: mutations,
        }
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

        doc_storage.process(Transaction::new(vec![
            // frame
            DocumentMutation::CreateObject(frame_id, ObjectKind::Frame),
            DocumentMutation::UpsertProp(frame_id, PropKind::PosX, Some(PropValue::Float(10.0))),
            DocumentMutation::UpsertProp(frame_id, PropKind::PosY, Some(PropValue::Float(20.0))),
            DocumentMutation::UpsertProp(
                frame_id,
                PropKind::Parent,
                Some(PropValue::Reference(document_id)),
            ),
            // oval
            DocumentMutation::CreateObject(oval_id, ObjectKind::Oval),
            DocumentMutation::UpsertProp(oval_id, PropKind::PosX, Some(PropValue::Float(100.0))),
            DocumentMutation::UpsertProp(oval_id, PropKind::PosY, Some(PropValue::Float(100.0))),
            DocumentMutation::UpsertProp(
                oval_id,
                PropKind::Parent,
                Some(PropValue::Reference(document_id)),
            ),
        ]));
        let snapshot = DocumentSnapshot::from(&doc_storage);
        let doc = ClientFollowerDocument::new(snapshot);

        let tx = convert_command_to_tx(
            &doc.storage,
            DocumentCommand::UpdateParent {
                id: oval_id,
                parent_id: frame_id,
            },
        )
        .expect("should work");

        let pos_x_after = tx
            .items
            .iter()
            .find_map(|m| match m {
                DocumentMutation::UpsertProp(
                    object_id,
                    PropKind::PosX,
                    Some(PropValue::Float(pos_x)),
                ) if object_id == &oval_id => Some(pos_x.clone()),
                _ => None,
            })
            .expect("must exist");
        assert_eq!(pos_x_after, 90.0);

        let pos_y_after = tx
            .items
            .iter()
            .find_map(|m| match m {
                DocumentMutation::UpsertProp(
                    object_id,
                    PropKind::PosY,
                    Some(PropValue::Float(pos_y)),
                ) if object_id == &oval_id => Some(pos_y.clone()),
                _ => None,
            })
            .expect("must exist");
        assert_eq!(pos_y_after, 80.0);
    }
}
