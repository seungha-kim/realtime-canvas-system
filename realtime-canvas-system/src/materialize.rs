use serde::{Deserialize, Serialize};

use crate::traits::{DocumentReadable, PropReadable};

use super::document_storage::DocumentStorage;
use super::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMaterial {
    id: uuid::Uuid,
    title: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum InvalidatedMaterial {
    Document,
}

pub trait Materialize<R: PropReadable + DocumentReadable> {
    fn readable(&self) -> &R;

    fn materialize_document(&self) -> DocumentMaterial {
        let readable = self.readable();
        let document_id = readable.document_id();
        let title = readable
            .get_string_prop(&PropKey(document_id.clone(), "title".into()))
            .unwrap_or("Untitled")
            .into();

        DocumentMaterial {
            id: document_id,
            title,
        }
    }
}
