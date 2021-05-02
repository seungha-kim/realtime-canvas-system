use serde::{Deserialize, Serialize};

use crate::traits::ReadableStorage;

use super::document_storage::DocumentStorage;
use super::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    title: String,
}

pub trait Materialize<R: ReadableStorage> {
    fn readable(&self) -> &R;

    fn document(&mut self, document_id: &ObjectId) -> Document {
        let title = self
            .readable()
            .get_string_prop(&PropKey(document_id.clone(), "title".into()))
            .unwrap_or("Untitled")
            .into();

        Document { title }
    }
}
