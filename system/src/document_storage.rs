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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentStorage {
    document_id: uuid::Uuid,
    objects: HashMap<ObjectId, ObjectKind>,
    deleted_objects: HashSet<ObjectId>,

    string_props: HashMap<PropKey, String>,
    float_props: HashMap<PropKey, f32>,
    color_props: HashMap<PropKey, Color>,
    reference_props: HashMap<PropKey, ObjectId>,
}

impl DocumentStorage {
    pub fn new() -> Self {
        let document_id = uuid::Uuid::new_v4();
        let mut objects = HashMap::new();
        objects.insert(document_id.clone(), ObjectKind::Document);
        DocumentStorage {
            document_id,
            objects,
            deleted_objects: HashSet::new(),

            string_props: HashMap::new(),
            float_props: HashMap::new(),
            color_props: HashMap::new(),
            reference_props: HashMap::new(),
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
            DocumentMutation::UpsertProp(prop_key, prop_value) => {
                match prop_value {
                    Some(PropValue::String(v)) => {
                        self.string_props.insert(prop_key.clone(), v.clone());
                    }
                    Some(PropValue::Float(v)) => {
                        self.float_props.insert(prop_key.clone(), v.clone());
                    }
                    Some(PropValue::Reference(id)) => {
                        self.reference_props.insert(prop_key.clone(), id.clone());
                    }
                    Some(PropValue::Color(color)) => {
                        self.color_props.insert(prop_key.clone(), color.clone());
                    }
                    None => {
                        self.string_props.remove(prop_key);
                        self.float_props.remove(prop_key);
                        self.reference_props.remove(prop_key);
                        self.color_props.remove(prop_key);
                    }
                }
                Ok(())
            }
            DocumentMutation::DeleteObject(object_id) => {
                if self.objects.contains_key(object_id) {
                    self.deleted_objects.insert(object_id.clone());
                }
                Ok(())
            }
        }
    }
}

impl PropReadable for DocumentStorage {
    fn get_string_prop(&self, key: &PropKey) -> Option<&str> {
        self.string_props.get(key).map(String::as_ref)
    }

    fn get_id_prop(&self, key: &PropKey) -> Option<&ObjectId> {
        self.reference_props.get(key)
    }

    fn get_float_prop(&self, key: &PropKey) -> Option<&f32> {
        self.float_props.get(key)
    }

    fn get_color_prop(&self, key: &PropKey) -> Option<&Color> {
        self.color_props.get(key)
    }

    fn get_object_kind(&self, object_id: &ObjectId) -> Option<&ObjectKind> {
        self.objects.get(object_id)
    }

    fn is_deleted(&self, object_id: &ObjectId) -> Option<bool> {
        Some(self.deleted_objects.contains(object_id))
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
