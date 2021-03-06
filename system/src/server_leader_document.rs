use crate::materialize::Materialize;
use crate::transactional_document::TransactionalDocument;

use super::message::*;
use crate::document::DocumentSnapshot;
use crate::traits::DocumentReadable;
use crate::Document;
use uuid::Uuid;

#[derive(Debug)]
pub struct ServerLeaderDocument {
    tx_document: TransactionalDocument,
}

impl Materialize<TransactionalDocument> for ServerLeaderDocument {
    fn readable(&self) -> &TransactionalDocument {
        &self.tx_document
    }
}

impl ServerLeaderDocument {
    pub fn new(document: Document) -> Self {
        Self {
            tx_document: TransactionalDocument::new(document),
        }
    }

    pub fn process_transaction(&mut self, tx: Transaction) -> Result<Transaction, ()> {
        let tx_id = tx.id;
        self.tx_document.begin(tx.clone());
        // TODO: validation
        self.tx_document.finish(&tx_id, true).expect("must finish");
        Ok(tx)
    }

    pub fn document(&self) -> &Document {
        self.tx_document.document()
    }
}

impl DocumentReadable for ServerLeaderDocument {
    fn document_id(&self) -> Uuid {
        unimplemented!()
    }

    fn snapshot(&self) -> DocumentSnapshot {
        self.tx_document.snapshot()
    }
}
