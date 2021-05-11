use serde::Serialize;

use crate::message::*;
use crate::traits::{DocumentReadable, PropReadable};

#[derive(Debug, Clone, Serialize)]
pub struct DocumentMaterial {
    id: uuid::Uuid,
    title: String,
}

pub trait Materialize<R: PropReadable + DocumentReadable> {
    fn readable(&self) -> &R;

    fn materialize_document(&self) -> DocumentMaterial {
        let readable = self.readable();
        let document_id = readable.document_id();
        let title = readable
            .get_string_prop(&PropKey(document_id.clone(), PropKind::Title))
            .unwrap_or("Untitled")
            .into();

        DocumentMaterial {
            id: document_id,
            title,
        }
    }
}
