use std::collections::{HashMap, HashSet};

use crate::traits::{DocumentReadable, PropReadable};

use crate::message::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// TODO: tree - cyclic reference detection
// TODO: LayeredStorage - 레이어링을 해야 하기 때문에 partial property 를 지원해야 한다.
//       이 때 mutation_id 까지 같이 고려할 것
// TODO: serialize to file

// atomicity 를 유지해야 하는 단위로 데이터를 저장하는 key value store. 그 밖에 대해서는 모른다. (undo 라던가)

type RecordId = uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    object_id: ObjectId,
    prop_kind: PropKind,
    prop_value: PropValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentStorage {
    document_id: uuid::Uuid,
    objects: HashMap<ObjectId, ObjectKind>,

    props: HashMap<RecordId, Record>,
    idx_by_object_id_and_prop_kind: HashMap<(ObjectId, PropKind), RecordId>,
    idx_by_object_id: HashMap<ObjectId, Vec<RecordId>>,
}

impl DocumentStorage {
    pub fn new() -> Self {
        let document_id = uuid::Uuid::new_v4();
        let mut objects = HashMap::new();
        objects.insert(document_id.clone(), ObjectKind::Document);
        DocumentStorage {
            document_id,
            objects,

            props: HashMap::new(),
            idx_by_object_id_and_prop_kind: HashMap::new(),
            idx_by_object_id: HashMap::new(),
        }
    }

    pub fn document_id(&self) -> uuid::Uuid {
        self.document_id
    }

    pub fn process(&mut self, tx: Transaction) -> Result<(), ()> {
        for m in &tx.items {
            // TODO: Err
            self.mutate(m).unwrap();
        }
        Ok(())
    }

    fn mutate(&mut self, mutation: &DocumentMutation) -> Result<(), ()> {
        match &mutation {
            DocumentMutation::CreateObject(object_id, object_kind) => {
                self.objects.insert(object_id.clone(), object_kind.clone());
                Ok(())
            }
            DocumentMutation::UpsertProp(object_id, prop_kind, prop_value_opt) => {
                if let Some(prop_value) = prop_value_opt {
                    if let Some(record_id) = self
                        .idx_by_object_id_and_prop_kind
                        .get(&(object_id.clone(), prop_kind.clone()))
                    {
                        // Update prop
                        let record = self.props.get_mut(record_id).expect("must exist");
                        *record = Record {
                            object_id: object_id.clone(),
                            prop_kind: prop_kind.clone(),
                            prop_value: prop_value.clone(),
                        };
                    } else {
                        // Insert prop
                        let record_id = uuid::Uuid::new_v4();
                        self.props.insert(
                            record_id,
                            Record {
                                object_id: object_id.clone(),
                                prop_kind: prop_kind.clone(),
                                prop_value: prop_value.clone(),
                            },
                        );
                        self.create_index_item(&record_id, object_id, prop_kind);
                    }
                } else {
                    // Delete prop
                    let idx_key = &(object_id.clone(), prop_kind.clone());
                    let record_id = self
                        .idx_by_object_id_and_prop_kind
                        .get(idx_key)
                        .expect("must exist")
                        .clone();
                    self.props.remove(&record_id);
                    self.delete_index_item(&record_id, object_id, prop_kind);
                }
                Ok(())
            }
            DocumentMutation::DeleteObject(object_id) => {
                self.objects.remove(object_id);
                Ok(())
            }
        }
    }

    fn create_index_item(
        &mut self,
        record_id: &RecordId,
        object_id: &ObjectId,
        prop_kind: &PropKind,
    ) {
        self.idx_by_object_id_and_prop_kind
            .insert((object_id.clone(), prop_kind.clone()), record_id.clone());

        if !self.idx_by_object_id.contains_key(object_id) {
            self.idx_by_object_id.insert(object_id.clone(), Vec::new());
        }
        let v = self
            .idx_by_object_id
            .get_mut(object_id)
            .expect("must exist");
        if !v.contains(record_id) {
            v.push(record_id.clone());
        }
    }

    fn delete_index_item(
        &mut self,
        record_id: &RecordId,
        object_id: &ObjectId,
        prop_kind: &PropKind,
    ) {
        self.idx_by_object_id_and_prop_kind
            .remove(&(object_id.clone(), prop_kind.clone()));

        let should_delete_vec = if let Some(v) = self.idx_by_object_id.get_mut(object_id) {
            v.retain(|r| r != record_id);
            v.is_empty()
        } else {
            false
        };
        if should_delete_vec {
            self.idx_by_object_id.remove(object_id);
        }
    }
}

impl PropReadable for DocumentStorage {
    fn get_prop(&self, object_id: &ObjectId, prop_kind: &PropKind) -> Option<&PropValue> {
        self.idx_by_object_id_and_prop_kind
            .get(&(object_id.clone(), prop_kind.clone()))
            .and_then(|record_id| self.props.get(record_id))
            .map(|record| &record.prop_value)
    }

    fn get_object_kind(&self, object_id: &ObjectId) -> Option<&ObjectKind> {
        self.objects.get(object_id)
    }

    fn is_deleted(&self, object_id: &ObjectId) -> Option<bool> {
        if self.objects.contains_key(object_id) {
            Some(false)
        } else {
            None
        }
    }

    fn get_all_props_of_object(&self, object_id: &ObjectId) -> Vec<(PropKind, Option<PropValue>)> {
        self.idx_by_object_id
            .get(object_id)
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|record_id| {
                self.props
                    .get(record_id)
                    .map(|record| (record.prop_kind.clone(), Some(record.prop_value.clone())))
            })
            .collect()
    }

    fn containing_objects(&self) -> Box<dyn Iterator<Item = &ObjectId> + '_> {
        Box::new(self.objects.keys())
    }
}

impl DocumentReadable for DocumentStorage {
    fn document_id(&self) -> Uuid {
        self.document_id
    }

    fn snapshot(&self) -> DocumentSnapshot {
        self.into()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DocumentSnapshot {
    content: Vec<u8>,
}

impl std::fmt::Debug for DocumentSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentSnapshot")
            .field("size", &self.content.len())
            .finish()
    }
}

impl From<&DocumentStorage> for DocumentSnapshot {
    fn from(d: &DocumentStorage) -> Self {
        DocumentSnapshot {
            content: bincode::serialize(d).unwrap(),
        }
    }
}

impl From<&DocumentSnapshot> for DocumentStorage {
    fn from(snapshot: &DocumentSnapshot) -> Self {
        bincode::deserialize(&snapshot.content).unwrap()
    }
}
