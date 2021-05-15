use serde::Serialize;

use crate::message::*;
use crate::traits::{DocumentReadable, PropReadable};

#[derive(Debug, Clone, Serialize)]
pub struct DocumentMaterial {
    id: ObjectId,
    name: String,
    children: Vec<ObjectId>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OvalMaterial {
    id: ObjectId,
    name: String,
    pos_x: f32,
    pos_y: f32,
    r_h: f32,
    r_v: f32,
}

pub trait Materialize<R: PropReadable + DocumentReadable> {
    fn readable(&self) -> &R;

    fn materialize_document(&self) -> DocumentMaterial {
        let readable = self.readable();
        let document_id = readable.document_id();
        let name = readable
            .get_string_prop(&PropKey(document_id.clone(), PropKind::Name))
            .unwrap_or("Untitled")
            .into();

        let mut children: Vec<ObjectId> = readable.get_children(&document_id).into_iter().collect();
        // TODO: 제대로된 기준으로 정렬해야 함
        children.sort();
        DocumentMaterial {
            id: document_id,
            name,
            children,
        }
    }

    fn materialize_oval(&self, object_id: &ObjectId) -> Result<OvalMaterial, ()> {
        let readable = self.readable();
        readable
            .get_object_kind(object_id)
            .filter(|k| k == &&ObjectKind::Oval)
            .map(|_| OvalMaterial {
                id: object_id.clone(),
                name: readable
                    .get_string_prop(&PropKey(object_id.clone(), PropKind::Name))
                    .unwrap_or("Untitled")
                    .into(),
                pos_x: readable
                    .get_float_prop(&PropKey(object_id.clone(), PropKind::PosX))
                    .cloned()
                    .unwrap_or(0.0),
                pos_y: readable
                    .get_float_prop(&PropKey(object_id.clone(), PropKind::PosY))
                    .cloned()
                    .unwrap_or(0.0),
                r_h: readable
                    .get_float_prop(&PropKey(object_id.clone(), PropKind::RadiusH))
                    .cloned()
                    .unwrap_or(10.0),
                r_v: readable
                    .get_float_prop(&PropKey(object_id.clone(), PropKind::RadiusV))
                    .cloned()
                    .unwrap_or(10.0),
            })
            .ok_or(())
    }
}
