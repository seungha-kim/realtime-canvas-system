use std::collections::VecDeque;

use crate::document_storage::DocumentStorage;
use crate::materialize::Materialize;

use super::message::*;
use super::types::*;

pub struct ClientLeaderDocument {
    doc_storage: DocumentStorage,
}

impl Materialize<DocumentStorage> for ClientLeaderDocument {
    fn readable(&self) -> &DocumentStorage {
        &self.doc_storage
    }
}

impl ClientLeaderDocument {
    pub fn new() -> Self {
        Self {
            doc_storage: DocumentStorage::new(),
        }
    }

    pub fn process(&mut self, tx: Transaction) -> Result<(), ()> {
        self.doc_storage.process(tx)
    }
}
