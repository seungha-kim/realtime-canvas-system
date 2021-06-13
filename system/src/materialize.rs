use serde::Serialize;

use crate::message::*;
use crate::traits::{DocumentReadable, PropReadable};

#[derive(Debug, Clone, Serialize)]
pub struct DocumentMaterial {
    pub id: ObjectId,
    pub name: String,
    pub children: Vec<ObjectId>,
}

#[derive(Debug, Clone, Serialize)]
pub enum ObjectMaterial {
    Document(DocumentMaterial),
    Oval(OvalMaterial),
    Frame(FrameMaterial),
}

#[derive(Debug, Clone, Serialize)]
pub struct OvalMaterial {
    id: ObjectId,
    name: String,
    pos_x: f32,
    pos_y: f32,
    r_h: f32,
    r_v: f32,
    fill_color: Color,
    index: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrameMaterial {
    id: ObjectId,
    name: String,
    pos_x: f32,
    pos_y: f32,
    w: f32,
    h: f32,
    index: String,
    children: Vec<ObjectId>,
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

        let children = readable
            .get_children_indices(&document_id)
            .iter()
            .map(|(object_id, _)| object_id.clone())
            .collect();

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
                fill_color: readable
                    .get_color_prop(&PropKey(object_id.clone(), PropKind::FillColor))
                    .cloned()
                    .unwrap_or(Color::default()),
                index: readable
                    .get_string_prop(&PropKey(object_id.clone(), PropKind::Index))
                    .unwrap_or("?")
                    .into(),
            })
            .ok_or(())
    }

    fn materialize_frame(&self, object_id: &ObjectId) -> Result<FrameMaterial, ()> {
        let readable = self.readable();
        readable
            .get_object_kind(object_id)
            .filter(|k| k == &&ObjectKind::Frame)
            .map(|_| FrameMaterial {
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
                w: readable
                    .get_float_prop(&PropKey(object_id.clone(), PropKind::Width))
                    .cloned()
                    .unwrap_or(10.0),
                h: readable
                    .get_float_prop(&PropKey(object_id.clone(), PropKind::Height))
                    .cloned()
                    .unwrap_or(10.0),
                index: readable
                    .get_string_prop(&PropKey(object_id.clone(), PropKind::Index))
                    .unwrap_or("?")
                    .into(),
                children: readable
                    .get_children_indices(&object_id)
                    .iter()
                    .map(|(object_id, _)| object_id.clone())
                    .collect(),
            })
            .ok_or(())
    }

    fn materialize_object(&self, object_id: &ObjectId) -> Result<ObjectMaterial, ()> {
        match self.readable().get_object_kind(object_id).unwrap() {
            ObjectKind::Document => Ok(ObjectMaterial::Document(self.materialize_document())),
            ObjectKind::Oval => Ok(ObjectMaterial::Oval(
                self.materialize_oval(object_id).unwrap(),
            )),
            ObjectKind::Frame => Ok(ObjectMaterial::Frame(
                self.materialize_frame(object_id).unwrap(),
            )),
        }
    }
}
