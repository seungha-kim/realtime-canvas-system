use wasm_bindgen::__rt::std::collections::HashSet;

use realtime_canvas_system::{
    serde_json, ClientReplicaDocument, DocumentCommand, DocumentSnapshot, Materialize, ObjectId,
    SessionEvent, SessionId, SessionSnapshot, Transaction,
};

pub struct SessionState {
    session_id: SessionId,
    session_snapshot: SessionSnapshot,
    session_snapshot_invalidated: bool,
    document: ClientReplicaDocument,
    invalidated_object_ids: HashSet<ObjectId>,
}

impl SessionState {
    pub fn new(
        session_id: SessionId,
        document_snapshot: DocumentSnapshot,
        session_snapshot: SessionSnapshot,
    ) -> Self {
        Self {
            session_id,
            document: ClientReplicaDocument::new(document_snapshot),
            session_snapshot,
            session_snapshot_invalidated: true,
            invalidated_object_ids: HashSet::new(),
        }
    }
    pub fn handle_session_event(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::TransactionAck(tx_id) => {
                self.document.handle_ack(&tx_id);
            }
            SessionEvent::TransactionNack(tx_id, _reason) => {
                self.document.handle_nack(&tx_id);
            }
            SessionEvent::OthersTransaction(tx) => {
                if let Ok(result) = self.document.handle_transaction(tx) {
                    for id in result.invalidated_object_ids {
                        self.invalidated_object_ids.insert(id);
                    }
                }
            }
            SessionEvent::SessionStateChanged(session_snapshot) => {
                self.session_snapshot = session_snapshot;
            }
            SessionEvent::Fragment(fragment) => {
                log::trace!("Fragment: {:?}", fragment);
            }
            _ => unimplemented!(),
        }
    }

    pub fn push_document_command(&mut self, json: String) -> Result<Transaction, ()> {
        let command = serde_json::from_str::<DocumentCommand>(&json).unwrap();

        if self.invalidated_object_ids.len() > 0 {
            log::warn!("invalidate_object_ids must be consumed for each command");
        }
        // TODO: Err
        if let Ok(result) = self.document.handle_command(command) {
            for invalidated_object_id in result.invalidated_object_ids {
                self.invalidated_object_ids.insert(invalidated_object_id);
            }
            Ok(result.transaction)
        } else {
            Err(())
        }
    }

    pub fn consume_invalidated_object_ids(&mut self) -> String {
        log::trace!(
            "Objects being invalidated: {:?}",
            self.invalidated_object_ids
        );
        let result = serde_json::to_string(&self.invalidated_object_ids).unwrap();
        self.invalidated_object_ids.clear();
        result
    }

    pub fn consume_latest_session_snapshot(&mut self) -> Option<String> {
        if self.session_snapshot_invalidated {
            return Some(serde_json::to_string(&self.session_snapshot).unwrap());
        } else {
            None
        }
    }

    pub fn materialize_document(&self) -> String {
        let document = self.document.materialize_document();
        serde_json::to_string(&document).unwrap()
    }

    pub fn materialize_session(&self) -> String {
        serde_json::to_string(&self.session_snapshot).unwrap()
    }
}
