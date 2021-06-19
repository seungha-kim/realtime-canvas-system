use std::collections::HashSet;

use crate::materialize::Materialize;
use crate::traits::DocumentReadable;
use crate::transactional_storage::TransactionalStorage;
use crate::{DocumentCommand, DocumentSnapshot, PropReadable};

use super::message::*;
use base95::Base95;
use euclid::default::Transform2D;
use std::str::FromStr;

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
        let tx = self.convert_command_to_tx(command)?;
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

    fn convert_command_to_tx(&self, command: DocumentCommand) -> Result<Transaction, ()> {
        match command {
            DocumentCommand::UpdateDocumentName { name } => {
                Ok(Transaction::new(vec![DocumentMutation::UpsertProp(
                    PropKey(self.storage.document_id(), PropKind::Name),
                    PropValue::String(name),
                )]))
            }
            DocumentCommand::CreateOval {
                pos,
                r_h,
                r_v,
                fill_color,
            } => {
                let id = uuid::Uuid::new_v4();
                // TODO: parent_id 입력 받기
                let parent_id = self.readable().document_id();
                let index = self.create_last_index_of_parent(&parent_id);

                Ok(Transaction::new(vec![
                    DocumentMutation::CreateObject(id, ObjectKind::Oval),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::Parent),
                        PropValue::Reference(parent_id),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::Index),
                        PropValue::String(index.to_string()),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::PosX),
                        PropValue::Float(pos.x),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::PosY),
                        PropValue::Float(pos.y),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::RadiusH),
                        PropValue::Float(r_h),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::RadiusV),
                        PropValue::Float(r_v),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::FillColor),
                        PropValue::Color(fill_color),
                    ),
                ]))
            }
            DocumentCommand::CreateFrame { pos, h, w } => {
                let id = uuid::Uuid::new_v4();
                // TODO: parent_id 입력 받기
                let parent_id = self.readable().document_id();
                let index = self.create_last_index_of_parent(&parent_id);

                // FIXME: 테스트용 oval
                let oval_id = uuid::Uuid::new_v4();

                Ok(Transaction::new(vec![
                    DocumentMutation::CreateObject(id, ObjectKind::Frame),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::Parent),
                        PropValue::Reference(parent_id),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::Index),
                        PropValue::String(index.to_string()),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::PosX),
                        PropValue::Float(pos.x),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::PosY),
                        PropValue::Float(pos.y),
                    ),
                    DocumentMutation::UpsertProp(PropKey(id, PropKind::Width), PropValue::Float(w)),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::Height),
                        PropValue::Float(h),
                    ),
                    // FIXME: 테스트용 Oval
                    DocumentMutation::CreateObject(oval_id, ObjectKind::Oval),
                    DocumentMutation::UpsertProp(
                        PropKey(oval_id, PropKind::Parent),
                        PropValue::Reference(id),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(oval_id, PropKind::Index),
                        PropValue::String(Base95::mid().to_string()),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(oval_id, PropKind::PosX),
                        PropValue::Float(0.0),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(oval_id, PropKind::PosY),
                        PropValue::Float(0.0),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(oval_id, PropKind::RadiusH),
                        PropValue::Float(30.0),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(oval_id, PropKind::RadiusV),
                        PropValue::Float(30.0),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(oval_id, PropKind::FillColor),
                        PropValue::Color(Color {
                            r: 50,
                            g: 50,
                            b: 50,
                        }),
                    ),
                ]))
            }
            DocumentCommand::UpdateName { id, name } => {
                Ok(Transaction::new(vec![DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::Name),
                    PropValue::String(name),
                )]))
            }
            DocumentCommand::UpdatePosition { id, pos } => Ok(Transaction::new(vec![
                DocumentMutation::UpsertProp(PropKey(id, PropKind::PosX), PropValue::Float(pos.x)),
                DocumentMutation::UpsertProp(PropKey(id, PropKind::PosY), PropValue::Float(pos.y)),
            ])),
            DocumentCommand::DeleteObject { id } => {
                Ok(Transaction::new(vec![DocumentMutation::DeleteObject(id)]))
            }
            DocumentCommand::UpdateIndex { id, int_index } => {
                let new_index_str = self
                    .storage
                    .get_id_prop(&PropKey(id, PropKind::Parent))
                    .ok_or(())
                    .and_then(|parent_id| {
                        let indices = self.storage.get_children_indices(&parent_id);
                        if int_index > indices.len() {
                            Err(())
                        } else if int_index == 0 {
                            Ok(Base95::avg_with_zero(&indices[0].1))
                        } else if int_index == indices.len() {
                            Ok(Base95::avg_with_one(&indices[indices.len() - 1].1))
                        } else {
                            Ok(Base95::avg(
                                &indices[int_index - 1].1,
                                &indices[int_index].1,
                            ))
                        }
                    })
                    .map(|new_index| new_index.to_string())?;

                Ok(Transaction::new(vec![DocumentMutation::UpsertProp(
                    PropKey(id, PropKind::Index),
                    PropValue::String(new_index_str),
                )]))
            }
            DocumentCommand::UpdateParent { id, parent_id } => {
                let index = self.create_last_index_of_parent(&parent_id);

                let current_global_transform = self.readable().get_global_transform(&id);
                let target_parent_global_transform =
                    self.readable().get_global_transform(&parent_id);
                let new_local_transform = current_global_transform.then(
                    &target_parent_global_transform
                        .inverse()
                        .unwrap_or(Transform2D::identity()),
                );

                Ok(Transaction::new(vec![
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::Parent),
                        PropValue::Reference(parent_id),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::Index),
                        PropValue::String(index.to_string()),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::PosX),
                        PropValue::Float(new_local_transform.m31),
                    ),
                    DocumentMutation::UpsertProp(
                        PropKey(id, PropKind::PosY),
                        PropValue::Float(new_local_transform.m32),
                    ),
                ]))
            }
            _ => unimplemented!(),
        }
    }

    fn invalidated_object_ids(&self, tx: &Transaction) -> HashSet<ObjectId> {
        let mut result = HashSet::new();
        for m in &tx.items {
            match m {
                DocumentMutation::UpsertProp(
                    PropKey(object_id, PropKind::Parent),
                    PropValue::Reference(parent_id),
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

    fn create_last_index_of_parent(&self, parent_id: &ObjectId) -> Base95 {
        let children = self.storage.get_children_indices(&parent_id);

        children
            .last()
            .and_then(|(last_child_id, _)| {
                self.storage
                    .get_string_prop(&PropKey(last_child_id.clone(), PropKind::Index))
            })
            // TODO: Base95::from_str 실패하는 경우에 대한 처리
            .and_then(|last_index_str| Base95::from_str(last_index_str).ok())
            .map(|last_index| Base95::avg_with_one(&last_index))
            .unwrap_or(Base95::mid())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
                    PropValue::Float(10.0),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(frame_id, PropKind::PosY),
                    PropValue::Float(20.0),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(frame_id, PropKind::Parent),
                    PropValue::Reference(document_id),
                ),
                // oval
                DocumentMutation::CreateObject(oval_id, ObjectKind::Oval),
                DocumentMutation::UpsertProp(
                    PropKey(oval_id, PropKind::PosX),
                    PropValue::Float(100.0),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(oval_id, PropKind::PosY),
                    PropValue::Float(100.0),
                ),
                DocumentMutation::UpsertProp(
                    PropKey(oval_id, PropKind::Parent),
                    PropValue::Reference(document_id),
                ),
            ]))
            .unwrap();
        let snapshot = DocumentSnapshot::from(&doc_storage);
        let doc = ClientFollowerDocument::new(snapshot);

        let tx = doc
            .convert_command_to_tx(DocumentCommand::UpdateParent {
                id: oval_id,
                parent_id: frame_id,
            })
            .unwrap();

        let pos_x_after = tx
            .items
            .iter()
            .find_map(|m| match m {
                DocumentMutation::UpsertProp(
                    PropKey(object_id, PropKind::PosX),
                    PropValue::Float(pos_x),
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
                    PropValue::Float(pos_y),
                ) if object_id == &oval_id => Some(pos_y.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(pos_y_after, 80.0);
    }
}
