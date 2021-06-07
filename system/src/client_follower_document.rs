use std::collections::HashSet;

use crate::materialize::Materialize;
use crate::traits::DocumentReadable;
use crate::transactional_storage::TransactionalStorage;
use crate::{DocumentCommand, DocumentSnapshot, PropReadable};

use super::message::*;
use base95::Base95;
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
                Ok(Transaction::new(vec![DocumentMutation::UpdateObject(
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
                    DocumentMutation::UpdateObject(
                        PropKey(id, PropKind::Parent),
                        PropValue::Reference(parent_id),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(id, PropKind::Index),
                        PropValue::String(index.to_string()),
                    ),
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
                    DocumentMutation::UpdateObject(
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
                    DocumentMutation::UpdateObject(
                        PropKey(id, PropKind::Parent),
                        PropValue::Reference(parent_id),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(id, PropKind::Index),
                        PropValue::String(index.to_string()),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(id, PropKind::PosX),
                        PropValue::Float(pos.x),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(id, PropKind::PosY),
                        PropValue::Float(pos.y),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(id, PropKind::Width),
                        PropValue::Float(w),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(id, PropKind::Height),
                        PropValue::Float(h),
                    ),
                    // FIXME: 테스트용 Oval
                    DocumentMutation::CreateObject(oval_id, ObjectKind::Oval),
                    DocumentMutation::UpdateObject(
                        PropKey(oval_id, PropKind::Parent),
                        PropValue::Reference(id),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(oval_id, PropKind::Index),
                        PropValue::String(index.to_string()),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(oval_id, PropKind::PosX),
                        PropValue::Float(0.0),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(oval_id, PropKind::PosY),
                        PropValue::Float(0.0),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(oval_id, PropKind::RadiusH),
                        PropValue::Float(30.0),
                    ),
                    DocumentMutation::UpdateObject(
                        PropKey(oval_id, PropKind::RadiusV),
                        PropValue::Float(30.0),
                    ),
                    DocumentMutation::UpdateObject(
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
                Ok(Transaction::new(vec![DocumentMutation::UpdateObject(
                    PropKey(id, PropKind::Name),
                    PropValue::String(name),
                )]))
            }
            DocumentCommand::UpdatePosition { id, pos } => Ok(Transaction::new(vec![
                DocumentMutation::UpdateObject(
                    PropKey(id, PropKind::PosX),
                    PropValue::Float(pos.x),
                ),
                DocumentMutation::UpdateObject(
                    PropKey(id, PropKind::PosY),
                    PropValue::Float(pos.y),
                ),
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

                Ok(Transaction::new(vec![DocumentMutation::UpdateObject(
                    PropKey(id, PropKind::Index),
                    PropValue::String(new_index_str),
                )]))
            }
            _ => unimplemented!(),
        }
    }

    fn invalidated_object_ids(&self, tx: &Transaction) -> HashSet<ObjectId> {
        tx.items
            .iter()
            .filter_map(|m| match m {
                DocumentMutation::UpdateObject(
                    PropKey(_, PropKind::Parent),
                    PropValue::Reference(parent_id),
                ) => Some(parent_id.clone()),
                DocumentMutation::UpdateObject(PropKey(object_id, PropKind::Index), _)
                | DocumentMutation::DeleteObject(object_id) => Some(
                    self.readable()
                        .get_id_prop(&PropKey(object_id.clone(), PropKind::Parent))
                        .unwrap()
                        .clone(),
                ),
                DocumentMutation::UpdateObject(prop_key, _) => Some(prop_key.0),
                _ => None,
            })
            .collect()
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