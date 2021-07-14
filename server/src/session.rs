use std::collections::VecDeque;
use system::{
    ConnectionId, Document, DocumentReadable, DocumentSnapshot, FileId, ServerLeaderDocument,
    SessionSnapshot, Transaction, TransactionId,
};

#[derive(Debug)]
struct PendingTransactionItem {
    from: ConnectionId,
    tx: Transaction,
}

#[derive(Debug)]
pub struct PendingTransactionCommitResult {
    pub from: ConnectionId,
    pub tx: Transaction,
}

#[derive(Debug)]
pub enum PendingTransactionCommitError {
    InvalidRequest,
    Rollback {
        from: ConnectionId,
        tx_id: TransactionId,
    },
}

#[derive(Debug)]
pub struct Session {
    pub file_id: FileId,
    pub connections: Vec<ConnectionId>,
    document: ServerLeaderDocument,
    pub behavior: SessionBehavior,
    pending_txs: VecDeque<PendingTransactionItem>,
}

#[derive(Debug, Clone)]
pub enum SessionBehavior {
    AutoTerminateWhenEmpty,
    ManualCommitByAdmin,
}

impl Session {
    pub fn new(file_id: FileId, document: Document, behavior: SessionBehavior) -> Self {
        Self {
            file_id,
            connections: Vec::new(),
            document: ServerLeaderDocument::new(document),
            behavior,
            pending_txs: VecDeque::new(),
        }
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            connections: self.connections.clone(),
        }
    }

    pub fn should_terminate(&self) -> bool {
        match self.behavior {
            SessionBehavior::AutoTerminateWhenEmpty => self.connections.is_empty(),
            _ => false,
        }
    }

    pub fn handle_transaction(
        &mut self,
        from: &ConnectionId,
        tx: Transaction,
    ) -> Result<Option<Transaction>, ()> {
        match self.behavior {
            SessionBehavior::AutoTerminateWhenEmpty => {
                self.document.process_transaction(tx).map(|tx| Some(tx))
            }
            SessionBehavior::ManualCommitByAdmin => {
                self.pending_txs.push_back(PendingTransactionItem {
                    from: from.clone(),
                    tx,
                });
                Ok(None)
            }
        }
    }

    pub fn commit_pending_transaction(
        &mut self,
    ) -> Result<Option<PendingTransactionCommitResult>, PendingTransactionCommitError> {
        match self.behavior {
            SessionBehavior::AutoTerminateWhenEmpty => {
                Err(PendingTransactionCommitError::InvalidRequest)
            }
            SessionBehavior::ManualCommitByAdmin => {
                if let Some(PendingTransactionItem { from, tx }) = self.pending_txs.pop_front() {
                    let tx_id = tx.id.clone();
                    self.document
                        .process_transaction(tx)
                        .map(|tx| Some(PendingTransactionCommitResult { from, tx }))
                        .map_err(|_| PendingTransactionCommitError::Rollback { from, tx_id })
                } else {
                    Err(PendingTransactionCommitError::InvalidRequest)
                }
            }
        }
    }

    pub fn document_snapshot(&self) -> DocumentSnapshot {
        self.document.snapshot()
    }

    pub fn document(&self) -> &Document {
        self.document.document()
    }

    pub fn has_pending_transactions(&self) -> bool {
        !self.pending_txs.is_empty()
    }
}

// TODO
