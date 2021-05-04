use serde::{Deserialize, Serialize};

use crate::traits::{DocumentReadable, PropReadable};

use super::document_storage::DocumentStorage;
use super::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    title: String,
}

pub trait Materialize<R: PropReadable + DocumentReadable> {
    fn readable(&self) -> &R;

    fn materialize_document(&self) -> Document {
        let readable = self.readable();
        let document_id = readable.document_id();
        let title = readable
            .get_string_prop(&PropKey(document_id.clone(), "title".into()))
            .unwrap_or("Untitled")
            .into();

        Document { title }
    }
}
