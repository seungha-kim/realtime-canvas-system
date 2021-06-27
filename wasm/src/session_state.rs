use wasm_bindgen::__rt::std::collections::HashSet;

use std::collections::VecDeque;
use system::{
    serde_json, ClientFollowerDocument, DocumentCommand, DocumentSnapshot, LivePointerEvent,
    Materialize, ObjectId, SessionEvent, SessionSnapshot, Transaction,
};

pub struct SessionState {
    session_snapshot: SessionSnapshot,
    session_snapshot_invalidated: bool,
    document: ClientFollowerDocument,
    invalidated_object_ids: HashSet<ObjectId>,
    pending_live_pointer_events: VecDeque<LivePointerEvent>,
}

impl SessionState {
    pub fn new(document_snapshot: DocumentSnapshot, session_snapshot: SessionSnapshot) -> Self {
        Self {
            document: ClientFollowerDocument::new(document_snapshot),
            session_snapshot,
            session_snapshot_invalidated: true,
            invalidated_object_ids: HashSet::new(),
            pending_live_pointer_events: VecDeque::new(),
        }
    }

    pub fn handle_session_event(&mut self, event: SessionEvent) -> Result<(), ()> {
        match event {
            SessionEvent::TransactionAck(tx_id) => self.document.handle_ack(&tx_id).map(|_| ()),
            SessionEvent::TransactionNack(tx_id, _reason) => {
                self.document.handle_nack(&tx_id).map(|_| ())
            }
            SessionEvent::OthersTransaction(tx) => {
                if let Ok(result) = self.document.handle_transaction(tx) {
                    for id in result.invalidated_object_ids {
                        self.invalidated_object_ids.insert(id);
                    }
                }
                Ok(())
            }
            SessionEvent::SessionStateChanged(session_snapshot) => {
                self.session_snapshot = session_snapshot;
                Ok(())
            }
            SessionEvent::LivePointer(live_pointer) => {
                if self.pending_live_pointer_events.len() > 100 {
                    log::warn!("live pointer events must be consumed")
                }
                self.pending_live_pointer_events.push_back(live_pointer);
                Ok(())
            }
            _ => unimplemented!(),
        }
    }

    pub fn push_document_command(&mut self, json: String) -> Result<Transaction, ()> {
        let command = serde_json::from_str::<DocumentCommand>(&json).map_err(|_| ())?;

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

    pub fn undo(&mut self) -> Result<Transaction, ()> {
        // TODO: Err
        if let Ok(result) = self.document.undo() {
            for invalidated_object_id in result.invalidated_object_ids {
                self.invalidated_object_ids.insert(invalidated_object_id);
            }
            Ok(result.transaction)
        } else {
            Err(())
        }
    }

    pub fn redo(&mut self) -> Result<Transaction, ()> {
        // TODO: Err
        if let Ok(result) = self.document.redo() {
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
        let result = serde_json::to_string(&self.invalidated_object_ids).unwrap_or("[]".into());
        self.invalidated_object_ids.clear();
        result
    }

    pub fn consume_latest_session_snapshot(&mut self) -> Option<String> {
        if self.session_snapshot_invalidated {
            serde_json::to_string(&self.session_snapshot).ok()
        } else {
            None
        }
    }

    pub fn consume_live_pointer_events(&mut self) -> String {
        serde_json::to_string(
            &self
                .pending_live_pointer_events
                .drain(..)
                .collect::<Vec<_>>(),
        )
        .expect("must succeed")
    }

    pub fn materialize_document(&self) -> String {
        let document = self.document.materialize_document();
        serde_json::to_string(&document).expect("must succeed")
    }

    pub fn materialize_session(&self) -> String {
        serde_json::to_string(&self.session_snapshot).expect("must succeed")
    }

    // TODO: 타입 바뀌었음
    pub fn materialize_object(&self, object_id: &ObjectId) -> Option<String> {
        self.document
            .materialize_object(&object_id)
            .map(|m| serde_json::to_string(&m).expect("must succeed"))
            .ok()
    }
}
