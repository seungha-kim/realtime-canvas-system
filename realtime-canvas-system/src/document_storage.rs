use std::collections::HashMap;

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
    objects: HashMap<ObjectId, ObjectType>,

    string_props: HashMap<PropKey, String>,
    float_props: HashMap<PropKey, f32>,
}

impl DocumentStorage {
    pub fn new() -> Self {
        DocumentStorage {
            document_id: uuid::Uuid::new_v4(),
            objects: HashMap::new(),

            string_props: HashMap::new(),
            float_props: HashMap::new(),
        }
    }

    pub fn document_id(&self) -> uuid::Uuid {
        self.document_id
    }

    pub fn process(&mut self, tx: Transaction) -> Result<(), ()> {
        for m in &tx.items {
            self.mutate(m);
        }
        Ok(())
    }

    fn mutate(&mut self, mutation: &DocumentMutation) -> Result<(), ()> {
        match &mutation {
            DocumentMutation::CreateObject(object_id, object_type) => {
                self.objects.insert(object_id.clone(), object_type.clone());
                Ok(())
            }
            DocumentMutation::UpdateObject(prop_key, prop_value) => {
                match prop_value {
                    PropValue::String(v) => {
                        self.string_props.insert(prop_key.clone(), v.clone());
                    }
                    PropValue::Float(v) => {
                        self.float_props.insert(prop_key.clone(), v.clone());
                    }
                }
                Ok(())
            }
            DocumentMutation::DeleteObject(_) => {
                // TODO: delete - prop 다 삭제, children 재귀적으로 다 삭제, 인가?
                Ok(())
            }
        }
    }
}

impl PropReadable for DocumentStorage {
    fn get_string_prop(&self, key: &PropKey) -> Option<&str> {
        self.string_props.get(key).map(String::as_ref)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSnapshot {
    content: Vec<u8>,
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
